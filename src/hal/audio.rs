//! Audio source abstraction + energy VAD.
//! Phase-1 mock: `MockMic` generates a periodic sine burst pattern so the
//! audio_task pipeline (capture → VAD → counter) can be proven with no I2S hw.
//! Real I2S MEMS mic will implement `AudioSource` and swap in unchanged.

/// Mono PCM source. `read` fills `buf` with i16 samples, returns count.
pub trait AudioSource {
	fn read(&mut self, buf: &mut [i16]) -> usize;
}

/// RMS energy threshold (Q15-ish). ~512 = quiet room, ~4096 = loud voice.
pub const VAD_THRESHOLD: u32 = 2048;

/// True if RMS energy of `buf` exceeds `VAD_THRESHOLD`.
pub fn vad_energy(buf: &[i16]) -> bool {
	if buf.is_empty() {
		return false;
	}
	let mut acc: u64 = 0;
	for &s in buf {
		let v = s as i32;
		acc = acc.saturating_add((v * v) as u64);
	}
	let mean_sq = acc / buf.len() as u64;
	mean_sq > VAD_THRESHOLD as u64 * VAD_THRESHOLD as u64
}

/// Mock mic: 8 kHz sine, quiet 200 samples then loud 80 samples, looping.
/// Loud burst RMS ~3277 → triggers VAD every ~35 ms at 8 kHz.
pub struct MockMic {
	pos: usize,
}

impl MockMic {
	pub const fn new() -> Self {
		Self { pos: 0 }
	}
}

const BURST_QUIET: usize = 200;
const BURST_LOUD: usize = 80;
const PERIOD: usize = BURST_QUIET + BURST_LOUD;

impl AudioSource for MockMic {
	fn read(&mut self, buf: &mut [i16]) -> usize {
		for slot in buf.iter_mut() {
			let loud = self.pos % PERIOD >= BURST_QUIET;
			let amp: i32 = if loud { 28000 } else { 200 };
			// square wave: 32-sample period, sign flips each 16 samples
			let s = if (self.pos / 16) & 1 == 0 { amp } else { -amp };
			*slot = s as i16;
			self.pos = self.pos.wrapping_add(1);
		}
		buf.len()
	}
}
