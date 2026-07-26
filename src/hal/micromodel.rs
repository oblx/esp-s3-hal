//! PLE1 micromodel loader + inference engine.
//!
//! Parses the flat binary artifact from s3-micromodel's export.py and runs
//! the PLE transformer forward pass in pure Rust (no_std, fp32 math).
//!
//! Weight tensors stay in the model blob (PSRAM). Activations use heap
//! scratch buffers (PSRAM). int4 weights are dequantized on-the-fly.
//!
//! Format: [magic u32] [8× i32 config] [f32 rope_theta] [tensors...]
//! Quant tensor: [i32 group] [int4 packed bytes] [fp16 scales]
//! FP32 tensor: [raw f32 bytes]

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use esp_println::println;
use libm::{sqrtf, powf, cosf, sinf, expf, tanhf};

fn fp16_to_f32(h: u16) -> f32 {
	if h == 0 { return 0.0; }
	let sign = if (h >> 15) & 1 != 0 { -1.0 } else { 1.0 };
	let exp = ((h >> 10) & 0x1F) as i32;
	let frac = (h & 0x3FF) as f32 / 1024.0;
	match exp {
		0 => sign * frac * powf(2.0, -14.0),
		31 => if frac == 0.0 { sign * f32::INFINITY } else { f32::NAN },
		_ => sign * (1.0 + frac) * powf(2.0, (exp - 15) as f32),
	}
}

const MAGIC: u32 = 0x504C4531;

/// Parsed PLE1 model configuration.
#[derive(Clone, Copy, Debug)]
pub struct ModelConfig {
	pub vocab_size: usize,
	pub d_model: usize,
	pub n_layers: usize,
	pub n_heads: usize,
	pub ffn_hidden: usize,
	pub ple_dim: usize,
	pub seq_len: usize,
	pub group: usize,
	pub rope_theta: f32,
}

impl ModelConfig {
	pub fn head_dim(&self) -> usize { self.d_model / self.n_heads }
}

/// A weight tensor — quantized (int4+fp16 scales) or raw fp32.
pub enum Weight {
	Quant { codes: &'static [u8], scales: &'static [u8], rows: usize, cols: usize, group: usize },
	Fp32(&'static [f32]),
}

impl Weight {
	fn quant_dims(&self) -> (usize, usize, usize) {
		match self {
			Weight::Quant { rows, cols, group, .. } => (*rows, *cols, *group),
			_ => (0, 0, 0),
		}
	}
}

struct LayerWeights {
	attn_norm: Weight,
	qkv: Weight,
	proj: Weight,
	ffn_norm: Weight,
	ffn_gate: Weight,
	ffn_up: Weight,
	ffn_down: Weight,
	ple_gate: Weight,
	ple_proj: Weight,
	ple_norm: Weight,
}

/// Parsed model — config + weight slices pointing into the model blob.
pub struct Model {
	pub cfg: ModelConfig,
	tok_emb: Weight,
	ple_model_proj: Weight,
	ple_proj_norm: Weight,
	ple_table: Weight,
	out_norm: Weight,
	layers: Box<[LayerWeights]>,
	_blob: Box<[u8]>,
}

fn rd_i32(buf: &[u8], off: usize) -> i32 {
	i32::from_le_bytes([buf[off], buf[off+1], buf[off+2], buf[off+3]])
}

/// Parse a PLE1 binary blob into a Model.
pub fn load(blob: Vec<u8>) -> Result<Model, &'static str> {
	if blob.len() < 40 { return Err("blob too small"); }
	let magic = u32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]);
	if magic != MAGIC { return Err("bad magic"); }
	let cfg = ModelConfig {
		vocab_size: rd_i32(&blob, 4) as usize,
		d_model: rd_i32(&blob, 8) as usize,
		n_layers: rd_i32(&blob, 12) as usize,
		n_heads: rd_i32(&blob, 16) as usize,
		ffn_hidden: rd_i32(&blob, 20) as usize,
		ple_dim: rd_i32(&blob, 24) as usize,
		seq_len: rd_i32(&blob, 28) as usize,
		group: rd_i32(&blob, 32) as usize,
		rope_theta: f32::from_bits(rd_i32(&blob, 36) as u32),
	};
	let mut blob = blob.into_boxed_slice();
	let static_ptr = blob.as_ptr();
	let static_len = blob.len();
	let static_blob: &'static [u8] = unsafe { core::slice::from_raw_parts(static_ptr, static_len) };
	let mut pos = 40usize;
	let mut rq = |pos: &mut usize, rows: usize, cols: usize| -> Result<Weight, &'static str> {
		let group = rd_i32(static_blob, *pos) as usize;
		*pos += 4;
		let rb = (cols + 1) / 2;
		let cl = rows * rb;
		if *pos + cl > static_blob.len() { return Err("trunc codes"); }
		let codes = &static_blob[*pos..*pos + cl];
		*pos += cl;
		let ng = (cols + group - 1) / group;
		let sl = rows * ng * 2;
		if *pos + sl > static_blob.len() { return Err("trunc scales"); }
		let scales = &static_blob[*pos..*pos + sl];
		*pos += sl;
		Ok(Weight::Quant { codes, scales, rows, cols, group })
	};
	let mut rf = |pos: &mut usize, n: usize| -> Result<Weight, &'static str> {
		let b = n * 4;
		if *pos + b > static_blob.len() { return Err("trunc fp32"); }
		let p = static_blob[*pos..].as_ptr() as *const f32;
		*pos += b;
		Ok(Weight::Fp32(unsafe { core::slice::from_raw_parts(p, n) }))
	};
	let tok_emb = rq(&mut pos, cfg.vocab_size, cfg.d_model)?;
	let ple_model_proj = rq(&mut pos, cfg.n_layers * cfg.ple_dim, cfg.d_model)?;
	let ple_proj_norm = rf(&mut pos, cfg.ple_dim)?;
	let ple_table = rq(&mut pos, cfg.vocab_size, cfg.n_layers * cfg.ple_dim)?;
	let mut layers = Vec::with_capacity(cfg.n_layers);
	for _ in 0..cfg.n_layers {
		layers.push(LayerWeights {
			attn_norm: rf(&mut pos, cfg.d_model)?,
			qkv: rq(&mut pos, 3 * cfg.d_model, cfg.d_model)?,
			proj: rq(&mut pos, cfg.d_model, cfg.d_model)?,
			ffn_norm: rf(&mut pos, cfg.d_model)?,
			ffn_gate: rq(&mut pos, cfg.ffn_hidden, cfg.d_model)?,
			ffn_up: rq(&mut pos, cfg.ffn_hidden, cfg.d_model)?,
			ffn_down: rq(&mut pos, cfg.d_model, cfg.ffn_hidden)?,
			ple_gate: rq(&mut pos, cfg.ple_dim, cfg.d_model)?,
			ple_proj: rq(&mut pos, cfg.d_model, cfg.ple_dim)?,
			ple_norm: rf(&mut pos, cfg.d_model)?,
		});
	}
	let out_norm = rf(&mut pos, cfg.d_model)?;
	println!("model loaded: {}L d={} V={} {}/{}", cfg.n_layers, cfg.d_model, cfg.vocab_size, pos, static_len);
	Ok(Model { cfg, tok_emb, ple_model_proj, ple_proj_norm, ple_table, out_norm, layers: layers.into_boxed_slice(), _blob: blob })
}

// ─── int4 dequant ──────────────────────────────────

fn dequant_row(out: &mut [f32], codes: &[u8], scales: &[u8], cols: usize, group: usize) {
	let ng = (cols + group - 1) / group;
	for gi in 0..ng {
		let a = gi * group;
		let b = core::cmp::min(a + group, cols);
		let sc = fp16_to_f32(u16::from_le_bytes([scales[gi*2], scales[gi*2+1]]));
		for j in a..b {
			let bi = j / 2;
			let nib = if j & 1 == 0 { codes[bi] & 0x0F } else { (codes[bi] >> 4) & 0x0F };
			out[j] = (nib as i32 - 8) as f32 * sc;
		}
	}
}

fn dequant_row_alloc(w: &Weight, row: usize) -> Vec<f32> {
	let (rows, cols, group) = w.quant_dims();
	let rb = (cols + 1) / 2;
	let ng = (cols + group - 1) / group;
	let (codes, scales) = match w {
		Weight::Quant { codes, scales, .. } => (codes, scales),
		_ => return Vec::new(),
	};
	let mut out = vec![0f32; cols];
	dequant_row(&mut out, &codes[row*rb..], &scales[row*ng*2..], cols, group);
	out
}

// ─── matmul (y = x @ W^T, W quantized) ─────────────

fn matmul_q(y: &mut [f32], x: &[f32], w: &Weight) {
	let (rows, cols, group) = w.quant_dims();
	let (codes, scales) = match w { Weight::Quant { codes, scales, .. } => (*codes, *scales), _ => return };
	let rb = (cols + 1) / 2;
	let ng = (cols + group - 1) / group;
	let mut wrow = vec![0f32; cols];
	for i in 0..rows {
		dequant_row(&mut wrow, &codes[i*rb..], &scales[i*ng*2..], cols, group);
		let mut acc = 0f32;
		for j in 0..cols { acc += x[j] * wrow[j]; }
		y[i] = acc;
	}
}

// ─── RMSNorm ───────────────────────────────────────

fn rmsnorm(out: &mut [f32], x: &[f32], w: &[f32], dim: usize) {
	let mut ms = 0f32;
	for i in 0..dim { ms += x[i] * x[i]; }
	let inv = 1.0 / sqrtf(ms / dim as f32 + 1e-6);
	for i in 0..dim { out[i] = x[i] * w[i] * inv; }
}

// ─── RoPE (split-half convention, matches model.py) ─────────

fn apply_rope(q: &mut [f32], pos: usize, head_dim: usize, theta: f32) {
	let half = head_dim / 2;
	for h in 0..half {
		let freq = 1.0 / powf(theta, (2 * h) as f32 / head_dim as f32);
		let angle = pos as f32 * freq;
		let (c, s) = (cosf(angle), sinf(angle));
		let (x1, x2) = (q[h], q[h + half]);
		q[h] = x1 * c - x2 * s;
		q[h + half] = x2 * c + x1 * s;
	}
}

fn silu(x: f32) -> f32 { x / (1.0 + expf(-x)) }
fn gelu(x: f32) -> f32 { 0.5 * x * (1.0 + tanhf(0.7978845608 * (x + 0.044715 * x * x * x))) }

// ─── Forward pass ──────────────────────────────────

impl Model {
	/// Run forward pass for token ids. Returns last-position logits.
	pub fn forward(&self, tokens: &[u32]) -> Vec<f32> {
		let cfg = self.cfg;
		let (d, t, hd, nh) = (cfg.d_model, tokens.len(), cfg.head_dim(), cfg.n_heads);
		let pt = cfg.n_layers * cfg.ple_dim;

		// Embedding
		let mut x = vec![0f32; t * d];
		for (ti, &tok) in tokens.iter().enumerate() {
			let row = dequant_row_alloc(&self.tok_emb, tok as usize);
			for j in 0..d { x[ti * d + j] = row[j]; }
		}

		// PLE context: (proj_norm(ple_model_proj(x)*d^-0.5) + table*sqrt(ple_dim)) * sqrt(0.5)
		// ple_proj_norm is RMSNorm(ple_dim) applied per-layer-slice, not on the full vector.
		let mut ple = vec![0f32; t * pt];
		for (ti, &tok) in tokens.iter().enumerate() {
			let mut proj = vec![0f32; pt];
			matmul_q(&mut proj, &x[ti*d..(ti+1)*d], &self.ple_model_proj);
			let scale = powf(d as f32, -0.5);
			for v in &mut proj { *v *= scale; }
			if let Weight::Fp32(nw) = &self.ple_proj_norm {
				let mut normed = vec![0f32; pt];
				// Per-slice RMSNorm: each ple_dim chunk normed independently
				for li in 0..cfg.n_layers {
					let off = li * cfg.ple_dim;
					rmsnorm(&mut normed[off..off+cfg.ple_dim], &proj[off..off+cfg.ple_dim], nw, cfg.ple_dim);
				}
				let tbl = dequant_row_alloc(&self.ple_table, tok as usize);
				let ts = sqrtf(cfg.ple_dim as f32);
				let merge = powf(2.0, -0.5);
				for j in 0..pt { ple[ti*pt+j] = (normed[j] + tbl[j]*ts) * merge; }
			}
		}

		// KV cache (PSRAM): n_layers × seq × 2 × d
		let mut kv = vec![0f32; cfg.n_layers * t * 2 * d];

		// Scratch
		let mut qkv = vec![0f32; 3 * d];
		let mut normed = vec![0f32; d];
		let mut attn_out = vec![0f32; d];
		let mut proj_out = vec![0f32; d];
		let mut gate_buf = vec![0f32; cfg.ffn_hidden];
		let mut up_buf = vec![0f32; cfg.ffn_hidden];
		let mut ffn_out = vec![0f32; d];
		let mut ple_g = vec![0f32; cfg.ple_dim];
		let mut ple_p = vec![0f32; d];
		let mut scores = vec![0f32; t];

		for (li, layer) in self.layers.iter().enumerate() {
			for ti in 0..t {
				let xt = &x[ti*d..(ti+1)*d];
				// ── Attention ──
				if let Weight::Fp32(nw) = &layer.attn_norm { rmsnorm(&mut normed, xt, nw, d); }
				matmul_q(&mut qkv, &normed, &layer.qkv);
				// RoPE on q and k (in-place in qkv)
				for h in 0..nh {
					let off = h * hd;
					apply_rope(&mut qkv[off..off+hd], ti, hd, cfg.rope_theta);
					apply_rope(&mut qkv[d+off..d+off+hd], ti, hd, cfg.rope_theta);
				}
				// Store K,V in cache
				let kv_off = li * t * 2 * d + ti * 2 * d;
				kv[kv_off..kv_off+d].copy_from_slice(&qkv[d..2*d]); // K
				kv[kv_off+d..kv_off+2*d].copy_from_slice(&qkv[2*d..3*d]); // V
				// Causal attention per head
				for h in 0..nh {
					let off = h * hd;
					// Scores: q · k_j for j=0..=ti
					let mut max_s = f32::NEG_INFINITY;
					for sj in 0..=ti {
						let k_off = li * t * 2 * d + sj * 2 * d + off;
						let mut dot = 0f32;
						for dh in 0..hd { dot += qkv[off+dh] * kv[k_off+dh]; }
						dot /= sqrtf(hd as f32);
						scores[sj] = dot;
						if dot > max_s { max_s = dot; }
					}
					// Softmax
					let mut sum = 0f32;
					for sj in 0..=ti { scores[sj] = expf(scores[sj]-max_s); sum += scores[sj]; }
					let inv = 1.0 / sum;
					// Weighted sum of V
					for dh in 0..hd {
						let mut acc = 0f32;
						for sj in 0..=ti {
							let v_off = li * t * 2 * d + sj * 2 * d + d + off;
							acc += scores[sj] * inv * kv[v_off+dh];
						}
						attn_out[off+dh] = acc;
					}
				}
				matmul_q(&mut proj_out, &attn_out, &layer.proj);
				for j in 0..d { x[ti*d+j] += proj_out[j]; }

				// ── FFN (SwiGLU) ──
				if let Weight::Fp32(nw) = &layer.ffn_norm { rmsnorm(&mut normed, &x[ti*d..(ti+1)*d], nw, d); }
				matmul_q(&mut gate_buf, &normed, &layer.ffn_gate);
				for j in 0..cfg.ffn_hidden { gate_buf[j] = silu(gate_buf[j]); }
				matmul_q(&mut up_buf, &normed, &layer.ffn_up);
				for j in 0..cfg.ffn_hidden { gate_buf[j] *= up_buf[j]; }
				matmul_q(&mut ffn_out, &gate_buf, &layer.ffn_down);
				for j in 0..d { x[ti*d+j] += ffn_out[j]; }

				// ── PLE per-layer ──
				let pl = &ple[ti*pt + li*cfg.ple_dim..ti*pt + (li+1)*cfg.ple_dim];
				matmul_q(&mut ple_g, &x[ti*d..(ti+1)*d], &layer.ple_gate);
				for j in 0..cfg.ple_dim { ple_g[j] = gelu(ple_g[j]) * pl[j]; }
				matmul_q(&mut ple_p, &ple_g, &layer.ple_proj);
				if let Weight::Fp32(nw) = &layer.ple_norm {
					let tmp = ple_p.clone();
					rmsnorm(&mut ple_p, &tmp, nw, d);
				}
				for j in 0..d { x[ti*d+j] += ple_p[j]; }
			}
		}

		// Output norm + head (tied with tok_emb)
		let mut last = vec![0f32; d];
		if let Weight::Fp32(nw) = &self.out_norm { rmsnorm(&mut last, &x[(t-1)*d..t*d], nw, d); }
		let mut logits = vec![0f32; cfg.vocab_size];
		for v in 0..cfg.vocab_size {
			let row = dequant_row_alloc(&self.tok_emb, v);
			let mut acc = 0f32;
			for j in 0..d { acc += last[j] * row[j]; }
			logits[v] = acc;
		}
		logits
	}
}
