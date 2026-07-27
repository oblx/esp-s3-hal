//! Flash collection parser — reads MODL manifest from flash via ROM SPI.
//!
//! Format: [u32 magic=0x4D4F444C] [u32 count] [entry0..entryN] [model blobs...]
//! Entry: name[32] arm[16] offset u32 size u32 vocab d_model n_layers n_heads
//!        ffn_hidden ple_dim seq_len group u32 val_ppl f32 params u32 (96 bytes)

use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;

/// MODL magic
pub const MAGIC: u32 = 0x4D4F444C;
/// PLE1 magic (model blob)
pub const PLE1_MAGIC: u32 = 0x504C4531;
/// Entry size in bytes
pub const ENTRY_SIZE: usize = 96;

/// ROM SPI flash read — 4-byte aligned addr/len.
unsafe extern "C" {
	fn esp_rom_spiflash_read(src_addr: u32, data: *const u32, len: u32) -> i32;
}

/// Read from flash via ROM, handling alignment padding.
pub unsafe fn flash_read(offset: u32, dst: &mut [u8]) -> bool {
	let pad = (4 - (offset % 4)) % 4;
	let read_offset = offset - pad;
	let read_len = ((dst.len() + pad as usize + 3) & !3) as u32;
	let mut buf = vec![0u8; read_len as usize];
	let ret = esp_rom_spiflash_read(read_offset, buf.as_mut_ptr() as *const u32, read_len);
	if ret != 0 { return false; }
	dst.copy_from_slice(&buf[pad as usize..pad as usize + dst.len()]);
	true
}

/// Model entry from the collection manifest.
#[derive(Clone, Debug)]
pub struct ModelEntry {
	pub name: String,
	pub arm: String,
	pub offset: u32,
	pub size: u32,
	pub vocab: u32,
	pub d_model: u32,
	pub n_layers: u32,
	pub n_heads: u32,
	pub ffn_hidden: u32,
	pub ple_dim: u32,
	pub seq_len: u32,
	pub group: u32,
	pub val_ppl: f32,
	pub params: u32,
}

fn rd_u32(buf: &[u8], off: usize) -> u32 {
	u32::from_le_bytes([buf[off], buf[off+1], buf[off+2], buf[off+3]])
}

fn rd_f32(buf: &[u8], off: usize) -> f32 {
	f32::from_le_bytes([buf[off], buf[off+1], buf[off+2], buf[off+3]])
}

fn rd_string(buf: &[u8], start: usize, len: usize) -> String {
	let slice = &buf[start..start + len];
	let end = slice.iter().position(|&b| b == 0).unwrap_or(len);
	String::from_utf8_lossy(&slice[..end]).into_owned()
}

/// Read the collection manifest from flash at `partition_offset`.
/// Returns a list of model entries. Empty list if no valid collection.
pub fn read_manifest(partition_offset: u32) -> Vec<ModelEntry> {
	unsafe {
		// Read header: magic + count
		let mut hdr = [0u8; 8];
		if !flash_read(partition_offset, &mut hdr) { return Vec::new(); }
		let magic = rd_u32(&hdr, 0);
		if magic != MAGIC { return Vec::new(); }
		let count = rd_u32(&hdr, 4) as usize;
		if count == 0 || count > 32 { return Vec::new(); }
		// Read all entries
		let entries_bytes = count * ENTRY_SIZE;
		let mut buf = vec![0u8; entries_bytes];
		if !flash_read(partition_offset + 8, &mut buf) { return Vec::new(); }
		let mut entries = Vec::with_capacity(count);
		for i in 0..count {
			let base = i * ENTRY_SIZE;
			entries.push(ModelEntry {
				name: rd_string(&buf, base, 32),
				arm: rd_string(&buf, base + 32, 16),
				offset: rd_u32(&buf, base + 48),
				size: rd_u32(&buf, base + 52),
				vocab: rd_u32(&buf, base + 56),
				d_model: rd_u32(&buf, base + 60),
				n_layers: rd_u32(&buf, base + 64),
				n_heads: rd_u32(&buf, base + 68),
				ffn_hidden: rd_u32(&buf, base + 72),
				ple_dim: rd_u32(&buf, base + 76),
				seq_len: rd_u32(&buf, base + 80),
				group: rd_u32(&buf, base + 84),
				val_ppl: rd_f32(&buf, base + 88),
				params: rd_u32(&buf, base + 92),
			});
		}
		entries
	}
}

/// Read a model blob from flash by entry.
pub fn read_model(partition_offset: u32, entry: &ModelEntry) -> Option<Vec<u8>> {
	unsafe {
		let mut buf = vec![0u8; entry.size as usize];
		let abs = partition_offset + entry.offset;
		if !flash_read(abs, &mut buf) { return None; }
		// Verify PLE1 magic
		if buf.len() < 4 { return None; }
		let magic = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
		if magic != PLE1_MAGIC { return None; }
		Some(buf)
	}
}
