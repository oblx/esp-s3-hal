//! Basic WS2812 LED color cycle using esp_s3_hal.
//!
//! Run: `cargo +esp build --release --example basic_led -Zbuild-std=core,compiler_builtins,alloc`

#![no_std]
#![no_main]

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

use esp_backtrace as _;
use esp_hal::delay::Delay;
use esp_hal::rmt::Rmt;
use esp_hal::time::Rate;
use esp_s3_hal::hal::led::{Led, LedCommand};

#[esp_hal::main]
fn main() -> ! {
	esp_println::println!("APP ALIVE");
	let peripherals = esp_hal::init(esp_hal::Config::default());
	esp_alloc::heap_allocator!(size: 64 * 1024);

	let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).expect("RMT init");
	let mut led = Led::new(rmt, peripherals.GPIO48);
	esp_println::println!("led ready on GPIO48");

	let colors = [
		("red", 255, 0, 0),
		("green", 0, 255, 0),
		("blue", 0, 0, 255),
		("off", 0, 0, 0),
	];
	let delay = Delay::new();
	let mut idx = 0;
	loop {
		let (name, r, g, b) = colors[idx];
		esp_println::println!("color {}", name);
		let cmd = LedCommand {
			state: if r + g + b > 0 { "on" } else { "off" }.into(),
			r, g, b, intensity: 0.8,
		};
		led.apply(cmd);
		idx = (idx + 1) % colors.len();
		delay.delay_millis(500);
	}
}
