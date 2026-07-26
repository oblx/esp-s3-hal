//! Runtime I2C bus control — master mode, async with timeout.
//!
//! Configure the bus once via `configure_bus(sda, scl, freq_khz)`,
//! then read/write to any device address on it.
//! Requires pull-up resistors (2.2k-10k) on SDA and SCL.

use core::sync::atomic::{AtomicU8, Ordering};

use esp_hal::gpio::AnyPin;
use esp_hal::i2c::master::{BusTimeout, Config, I2c};
use esp_hal::peripherals::I2C0;
use esp_hal::time::Rate;

extern crate alloc;

static SDA_PIN: AtomicU8 = AtomicU8::new(0);
static SCL_PIN: AtomicU8 = AtomicU8::new(0);
static FREQ_KHZ: AtomicU8 = AtomicU8::new(100);

/// Configure the I2C bus — call once before read/write.
pub fn configure_bus(sda_pin: u8, scl_pin: u8, freq_khz: u8) -> Result<(), &'static str> {
	crate::hal::gpio::check_pin(sda_pin)?;
	crate::hal::gpio::check_pin(scl_pin)?;
	if sda_pin == scl_pin {
		return Err("SDA and SCL must be different pins");
	}
	SDA_PIN.store(sda_pin, Ordering::Relaxed);
	SCL_PIN.store(scl_pin, Ordering::Relaxed);
	FREQ_KHZ.store(freq_khz, Ordering::Relaxed);
	Ok(())
}

/// Check if bus is configured.
pub fn is_configured() -> bool {
	SDA_PIN.load(Ordering::Relaxed) != 0
}

/// Get stored bus config.
pub fn bus_config() -> (u8, u8, u8) {
	(
		SDA_PIN.load(Ordering::Relaxed),
		SCL_PIN.load(Ordering::Relaxed),
		FREQ_KHZ.load(Ordering::Relaxed),
	)
}

fn make_i2c() -> Result<I2c<'static, esp_hal::Async>, &'static str> {
	if !is_configured() {
		return Err("I2C bus not configured — POST /i2c/config first");
	}
	let sda = unsafe { AnyPin::steal(SDA_PIN.load(Ordering::Relaxed)) };
	let scl = unsafe { AnyPin::steal(SCL_PIN.load(Ordering::Relaxed)) };
	let freq_khz = FREQ_KHZ.load(Ordering::Relaxed);
	let config = Config::default()
		.with_frequency(Rate::from_khz(freq_khz as u32))
		.with_timeout(BusTimeout::Maximum);
	I2c::new(unsafe { I2C0::steal() }, config)
		.map(|i2c| i2c.into_async())
		.map_err(|_| "I2C init failed")
}

/// Write bytes to a register on an I2C device. Async with 500ms timeout.
pub async fn write_reg(addr: u8, reg: u8, data: &[u8]) -> Result<(), &'static str> {
	if data.len() > 64 {
		return Err("data too long (max 64 bytes)");
	}
	let mut i2c = make_i2c()?;
	let mut buf = [0u8; 65];
	buf[0] = reg;
	buf[1..=data.len()].copy_from_slice(data);

	let result = embassy_time::with_timeout(
		embassy_time::Duration::from_millis(500),
		i2c.write_read_async(addr, &buf[..=data.len()], &mut []),
	).await;

	match result {
		Ok(Ok(())) => Ok(()),
		Ok(Err(_)) => Err("I2C write failed (NACK or bus error)"),
		Err(_) => Err("I2C write timeout (500ms)"),
	}
}

/// Read `len` bytes from a register on an I2C device. Async with 500ms timeout.
pub async fn read_reg(addr: u8, reg: u8, len: usize) -> Result<alloc::vec::Vec<u8>, &'static str> {
	if len > 64 {
		return Err("len too long (max 64 bytes)");
	}
	let mut i2c = make_i2c()?;
	let mut buf = alloc::vec![0u8; len];

	let result = embassy_time::with_timeout(
		embassy_time::Duration::from_millis(500),
		i2c.write_read_async(addr, &[reg], &mut buf),
	).await;

	match result {
		Ok(Ok(())) => Ok(buf),
		Ok(Err(_)) => Err("I2C read failed (NACK or bus error)"),
		Err(_) => Err("I2C read timeout (500ms)"),
	}
}
