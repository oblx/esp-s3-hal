use alloc::string::String;
use esp_hal::rmt::Rmt;
use ws2812_rmt::{Ws2812, Timing, buffer_len, RGB8};

const LED_COUNT: usize = 1;

#[derive(Clone, Copy, Debug, Default)]
pub struct LedState {
	pub on: bool,
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub level: u8,
}

#[derive(Debug, miniserde::Deserialize)]
pub struct LedCommand {
	pub state: String,
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub intensity: f64,
}

impl LedCommand {
	pub fn on(&self) -> bool {
		self.state.as_str() == "on"
	}

	pub fn level(&self) -> u8 {
		let v = (self.intensity * 255.0) as i32;
		v.clamp(0, 255) as u8
	}
}

type LedDriver = Ws2812<'static, { buffer_len(LED_COUNT) }>;

pub struct Led {
	driver: LedDriver,
}

impl Led {
	pub fn new<O>(rmt: Rmt<'static, esp_hal::Blocking>, pin: O) -> Self
	where
		O: esp_hal::gpio::interconnect::PeripheralOutput<'static>,
	{
		let driver = Ws2812::new(
			rmt.channel0, pin,
			Timing::WS2812B_AT_12_5NS_TICK_ESP32C3, 1,
		).expect("LED init");
		Self { driver }
	}

	pub fn apply(&mut self, cmd: LedCommand) {
		let s = cmd.level() as u16;
		let rgb = if cmd.on() {
			RGB8 {
				r: ((cmd.r as u16 * s) / 255) as u8,
				g: ((cmd.g as u16 * s) / 255) as u8,
				b: ((cmd.b as u16 * s) / 255) as u8,
			}
		} else {
			RGB8::default()
		};
		let _ = self.driver.write(&[rgb]);
	}
}
