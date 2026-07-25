/// Disable RTC brown-out detector.
///
/// Wi-Fi TX draws 300-500mA spikes that trip the BOD on weak USB power.
/// Safety net only — use a stable power supply (powered hub / USB 3.0).
pub fn disable() {
	unsafe {
		let rtc = esp32s3::RTC_CNTL::steal();
		rtc.brown_out().modify(|_, w| {
			w.brown_out_rst_ena().clear_bit();
			w.brown_out_ena().clear_bit()
		});
	}
}
