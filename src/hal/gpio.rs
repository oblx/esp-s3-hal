//! Runtime GPIO control — digital read/write/toggle over any pin.
//!
//! Uses `Flex` (mode-switchable pin driver) + `AnyPin::steal` for runtime
//! pin access without taking ownership from `Peripherals`.
//!
//! Reserved pins (runtime guard rejects):
//!   0, 3, 26-32 (flash/PSRAM), 45, 46 (strapping), 48 (WS2812 LED)

use esp_hal::gpio::{AnyPin, Flex, InputConfig, Level, OutputConfig, Pull};

extern crate alloc;
use alloc::vec::Vec;

/// Pins reserved by the board (strapping, flash/PSRAM, WS2812).
/// Runtime guard rejects operations on these.
const RESERVED: &[u8] = &[0, 3, 26, 27, 28, 29, 30, 31, 32, 45, 46, 48];

/// Max GPIO number on ESP32-S3 (0..=48, but 22-25 don't exist).
const MAX_PIN: u8 = 48;

/// Valid GPIO numbers on ESP32-S3 (0-21, 26-48, skipping 22-25).
fn is_valid_pin(n: u8) -> bool {
	matches!(n, 0..=21 | 26..=48)
}

fn is_reserved(n: u8) -> bool {
	RESERVED.contains(&n)
}

/// Check if a pin is available for user control.
/// Returns Err with a reason string if reserved or invalid.
pub fn check_pin(n: u8) -> Result<(), &'static str> {
	if !is_valid_pin(n) {
		return Err("invalid pin number (S3: 0-21, 26-48)");
	}
	if is_reserved(n) {
		return Err("reserved pin (strapping/flash/PSRAM/LED)");
	}
	Ok(())
}

/// Pin mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinMode {
	Input,
	Output,
}

/// Pin state as a string for JSON serialization.
pub fn level_str(l: Level) -> &'static str {
	match l {
		Level::High => "high",
		Level::Low => "low",
	}
}

/// Parse a level string ("high"/"low"/"toggle").
pub fn parse_action(s: &str) -> Option<PinAction> {
	match s {
		"high" => Some(PinAction::High),
		"low" => Some(PinAction::Low),
		"toggle" => Some(PinAction::Toggle),
		_ => None,
	}
}

/// Pin action from HTTP request.
#[derive(Debug, Clone, Copy)]
pub enum PinAction {
	High,
	Low,
	Toggle,
}

/// Read a pin's input level. Enables input buffer if needed.
pub fn read_pin(n: u8) -> Result<Level, &'static str> {
	check_pin(n)?;
	let mut pin = unsafe { Flex::new(AnyPin::steal(n)) };
	pin.set_input_enable(true);
	Ok(pin.level())
}

/// Write to a pin (high/low/toggle). Enables output if needed.
pub fn write_pin(n: u8, action: PinAction) -> Result<Level, &'static str> {
	check_pin(n)?;
	let mut pin = unsafe { Flex::new(AnyPin::steal(n)) };
	pin.apply_output_config(&OutputConfig::default());
	pin.set_output_enable(true);
	match action {
		PinAction::High => pin.set_high(),
		PinAction::Low => pin.set_low(),
		PinAction::Toggle => pin.toggle(),
	}
	Ok(pin.output_level())
}

/// Set pin as input (disables output).
pub fn set_input(n: u8) -> Result<(), &'static str> {
	check_pin(n)?;
	let mut pin = unsafe { Flex::new(AnyPin::steal(n)) };
	pin.set_output_enable(false);
	pin.apply_input_config(&InputConfig::default().with_pull(Pull::None));
	pin.set_input_enable(true);
	Ok(())
}

/// Set pin as output (disables input).
pub fn set_output(n: u8, initial: Level) -> Result<(), &'static str> {
	check_pin(n)?;
	let mut pin = unsafe { Flex::new(AnyPin::steal(n)) };
	pin.apply_output_config(&OutputConfig::default());
	pin.set_level(initial);
	pin.set_output_enable(true);
	Ok(())
}

/// List all user-available pins (valid, not reserved).
pub fn available_pins() -> Vec<u8> {
	let mut v = Vec::new();
	for n in 0..=MAX_PIN {
		if check_pin(n).is_ok() {
			v.push(n);
		}
	}
	v
}

/// Reset all user-available pins to input mode (safe default).
/// Disables output, enables input, no pull resistors.
/// Skips reserved pins (strapping, flash/PSRAM, LED).
pub fn reset_all() -> Vec<u8> {
	let mut reset = Vec::new();
	for n in 0..=MAX_PIN {
		if check_pin(n).is_ok() {
			if set_input(n).is_ok() {
				reset.push(n);
			}
		}
	}
	reset
}
