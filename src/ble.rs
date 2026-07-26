//! BLE scaffold — esp-radio HCI connector wrapper.
//!
//! Initializes the BLE controller and provides a `BleConnector` for
//! HCI read/write. This is the transport layer — GATT services and
//! audio streaming are built on top of this in the firmware.
//!
//! Requires `ble` feature on esp-radio (enabled via the `ble` feature here).

use esp_println::println;
use esp_radio::ble::controller::{BleConnector, BleInitError};
use esp_radio::ble::Config;

/// Initialize BLE controller with default config.
/// Returns a `BleConnector` for HCI communication.
pub fn init() -> Result<BleConnector<'static>, BleInitError> {
	let bt = unsafe { esp_hal::peripherals::BT::steal() };
	let connector = BleConnector::new(bt, Config::default())?;
	println!("BLE initialized (HCI connector ready)");
	Ok(connector)
}

/// Check if BLE feature is compiled in.
pub fn is_enabled() -> bool {
	cfg!(feature = "ble")
}
