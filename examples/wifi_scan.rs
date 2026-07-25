//! Wi-Fi scan example — scan nearby APs and print SSID + RSSI.
//!
//! Run: `cargo +esp build --release --no-default-features --features wifi-sta --example wifi_scan -Zbuild-std=core,compiler_builtins,alloc`

#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_radio::wifi::scan::ScanConfig;

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
	println!("wifi_scan start");
	let peripherals = esp_hal::init(esp_hal::Config::default());

	esp_alloc::heap_allocator!(size: 128 * 1024);

	let timg0 = TimerGroup::new(peripherals.TIMG0);
	let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
	esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);
	println!("rtos started");

	let mut ctrl = esp_radio::wifi::WifiController::new(
		peripherals.WIFI,
		esp_radio::wifi::ControllerConfig::default(),
	).expect("wifi controller");
	println!("controller ready (station mode)");

	let cfg = ScanConfig::default().with_max(15);
	println!("scanning...");
	match ctrl.scan_async(&cfg).await {
		Ok(results) => {
			println!("found {} APs:", results.len());
			for ap in &results {
				println!("  {:32} ch{:2} rssi{:4} {:?}",
					ap.ssid.as_str(), ap.channel, ap.signal_strength, ap.auth_method);
			}
		}
		Err(e) => println!("scan error: {:?}", e),
	}

	println!("scan done — halting");
	loop {
		embassy_time::Timer::after(embassy_time::Duration::from_secs(5)).await;
	}
}
