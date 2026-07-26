//! Mock mic + energy VAD demo using esp_s3_hal::hal::audio.
//!
//! Run: `cargo +esp build --release --no-default-features --example audio_vad -Zbuild-std=core,compiler_builtins,alloc`
//! No I2S hw required — MockMic generates a burst pattern in software.
//! (`--no-default-features` avoids pulling wifi-ap / esp-radio, like basic_led.)

#![no_std]
#![no_main]

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_backtrace as _;
use esp_hal::delay::Delay;
use esp_s3_hal::hal::audio::{AudioSource, MockMic, vad_energy};

#[esp_hal::main]
fn main() -> ! {
	esp_println::println!("APP ALIVE — audio_vad mock");
	let peripherals = esp_hal::init(esp_hal::Config::default());
	esp_alloc::heap_allocator!(size: 64 * 1024);
	let _ = peripherals; // no peripheral needed for mock
	let mut mic = MockMic::new();
	let mut buf = [0i16; 128];
	let mut events: u32 = 0;
	let delay = Delay::new();
	loop {
		let n = mic.read(&mut buf);
		if vad_energy(&buf[..n]) {
			events = events.wrapping_add(1);
			esp_println::println!("vad event #{} (rms burst)", events);
		}
		delay.delay_millis(20);
	}
}
