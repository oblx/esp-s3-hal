//! Runtime PWM control via LEDC (LED PWM Controller).
//!
//! Uses `LEDC::steal()` for runtime access. Each call configures a
//! fresh timer+channel. 8 low-speed channels (0-7) on ESP32-S3.

use esp_hal::gpio::{AnyPin, DriveMode};
use esp_hal::ledc::channel::{self, ChannelIFace, Number as ChNum};
use esp_hal::ledc::timer::{self, TimerIFace, Number as TimerNum};
use esp_hal::ledc::{LowSpeed, LSGlobalClkSource, Ledc};
use esp_hal::peripherals::LEDC;
use esp_hal::time::Rate;

/// Configure a PWM channel: freq (Hz), duty (0-100%), on a given GPIO.
pub fn set_pwm(pin: u8, channel: u8, freq_hz: u32, duty_pct: u8) -> Result<(), &'static str> {
	crate::hal::gpio::check_pin(pin)?;
	if channel > 7 {
		return Err("channel must be 0-7");
	}

	let mut ledc = unsafe { Ledc::new(LEDC::steal()) };
	ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);

	let timer_num = match channel {
		0 => TimerNum::Timer0,
		1 => TimerNum::Timer1,
		2 => TimerNum::Timer2,
		_ => TimerNum::Timer3,
	};

	let mut timer = ledc.timer::<LowSpeed>(timer_num);
	timer.configure(timer::config::Config {
		duty: timer::config::Duty::Duty10Bit,
		clock_source: timer::LSClockSource::APBClk,
		frequency: Rate::from_hz(freq_hz),
	}).map_err(|_| "timer config failed")?;

	let ch_num = match channel {
		0 => ChNum::Channel0,
		1 => ChNum::Channel1,
		2 => ChNum::Channel2,
		3 => ChNum::Channel3,
		4 => ChNum::Channel4,
		5 => ChNum::Channel5,
		6 => ChNum::Channel6,
		_ => ChNum::Channel7,
	};

	let output_pin = unsafe { AnyPin::steal(pin) };
	let mut ch = ledc.channel::<LowSpeed>(ch_num, output_pin);
	ch.configure(channel::config::Config {
		timer: &timer,
		duty_pct: duty_pct.min(100),
		drive_mode: DriveMode::PushPull,
	}).map_err(|_| "channel config failed")?;

	Ok(())
}

/// Stop a PWM channel — sets duty to 0 via direct register access.
pub fn stop_pwm(channel: u8) -> Result<(), &'static str> {
	set_duty(channel, 0)
}

/// Update duty cycle on an already-configured channel via direct register access.
/// Does NOT re-init LEDC (avoids peripheral reset). Duty 10-bit (0-1023).
pub fn set_duty(channel: u8, duty_pct: u8) -> Result<(), &'static str> {
	if channel > 7 {
		return Err("channel must be 0-7");
	}

	let ledc = esp_hal::peripherals::LEDC::regs();
	let ch = ledc.ch(channel as usize);
	let duty_val = ((duty_pct.min(100) as u32) * 1023) / 100;

	// Write duty (shifted left by 4 per LEDC hardware spec)
	ch.duty().write(|w| unsafe { w.duty().bits(duty_val << 4) });

	// Start duty without fading
	ch.conf1().write(|w| {
		w.duty_start().set_bit();
		w.duty_inc().set_bit();
		unsafe {
			w.duty_num().bits(0x1);
			w.duty_cycle().bits(0x1);
			w.duty_scale().bits(0x0)
		}
	});

	// Update channel (para_up)
	ch.conf0().modify(|_, w| w.para_up().set_bit());

	Ok(())
}
