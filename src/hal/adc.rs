//! Runtime ADC reader — ADC1 only (ADC2 conflicts with Wi-Fi).
//!
//! ADC1 channels on ESP32-S3:
//!   GPIO1=CH0, GPIO2=CH1, GPIO4=CH3, GPIO5=CH4, GPIO6=CH5,
//!   GPIO7=CH6, GPIO8=CH7, GPIO9=CH8, GPIO10=CH9

use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::peripherals::{ADC1, GPIO1, GPIO10, GPIO2, GPIO4, GPIO5, GPIO6, GPIO7, GPIO8, GPIO9};

/// Check if a pin is an ADC1 channel. Returns the channel number.
pub fn check_adc_pin(pin: u8) -> Result<u8, &'static str> {
	match pin {
		1 => Ok(0),
		2 => Ok(1),
		4 => Ok(3),
		5 => Ok(4),
		6 => Ok(5),
		7 => Ok(6),
		8 => Ok(7),
		9 => Ok(8),
		10 => Ok(9),
		_ => Err("not an ADC1 pin (ADC1: GPIO1,2,4-10)"),
	}
}

/// List all ADC1 pins.
pub fn adc_pins() -> &'static [u8] {
	&[1, 2, 4, 5, 6, 7, 8, 9, 10]
}

/// Read ADC1 for a given GPIO pin. Returns millivolts (0-3300).
/// Single sample, 11dB attenuation (0-3.3V range).
pub fn read_mv(pin: u8) -> Result<u16, &'static str> {
	read_mv_avg(pin, 1)
}

/// Read ADC1 with multi-sample averaging.
/// `samples` = number of reads to average (1-16, clamped).
/// Returns millivolts (0-3300).
pub fn read_mv_avg(pin: u8, samples: u8) -> Result<u16, &'static str> {
	let n = samples.clamp(1, 16) as u32;
	let adc1 = unsafe { ADC1::steal() };
	let mut config: AdcConfig<ADC1> = AdcConfig::new();

	macro_rules! do_read {
		($gpio:ident) => {{
			let mut p = config.enable_pin(unsafe { $gpio::steal() }, Attenuation::_11dB);
			let mut adc = Adc::new(adc1, config);
			let mut total: u32 = 0;
			for _ in 0..n {
				let raw: u16 = nb::block!(adc.read_oneshot(&mut p))
					.map_err(|_| "ADC read failed")?;
				total += raw as u32;
			}
			total / n
		}};
	}

	let avg_raw: u32 = match pin {
		1 => do_read!(GPIO1),
		2 => do_read!(GPIO2),
		4 => do_read!(GPIO4),
		5 => do_read!(GPIO5),
		6 => do_read!(GPIO6),
		7 => do_read!(GPIO7),
		8 => do_read!(GPIO8),
		9 => do_read!(GPIO9),
		10 => do_read!(GPIO10),
		_ => return Err("not an ADC1 pin"),
	};

	// 12-bit ADC (0-4095) → 0-3300mV (with 11dB attenuation)
	Ok(((avg_raw * 3300) / 4095) as u16)
}
