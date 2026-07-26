# HAL
## WS2812 RGB LED
- Pin: **GPIO48**
- Driver: `ws2812-rmt` 0.2.0 (RMT channel 0)
- Count: 1 LED
```rust
use esp_s3_hal::hal::led::{Led, LedCommand};

let mut led = Led::new(rmt, peripherals.GPIO48);
led.apply(LedCommand {
	state: "on".into(), r: 255, g: 0, b: 0, intensity: 0.8,
}).await;
```
## Why not `esp-hal-smartled2`
`esp-hal-smartled2` 0.28.2 has a timing bug: the `* 2` multiplier on pulse
durations produces wrong WS2812 signals with `esp-hal` 1.1.1.
Symptom: LED stays white or unresponsive.
`ws2812-rmt` uses correct timing + explicit reset pulses.
## LedCommand / LedState
| struct | fields | direction |
|-|-|-|
| `LedCommand` | state, r, g, b, intensity | web → led (channel) |
| `LedState` | on, r, g, b, level | shared (mutex) |
Intensity `0.0..1.0` → level `0..255` (clamp). Off = RGB(0,0,0).
## Brownout
`boot::brownout::disable` clears `RTC_CNTL.brown_out` RST + ENA bits.
Safety net for weak USB power during Wi-Fi TX spikes (300-500mA).
## Audio
`hal::audio` — `AudioSource` trait + `MockMic` + `vad_energy`.
Phase-1 mock: no I2S hw. `MockMic` emits a square-wave burst pattern
(quiet 200 / loud 80 samples) so the capture → VAD → counter pipeline
can be proven. Real I2S MEMS mic will implement `AudioSource` and swap in.
```rust
use esp_s3_hal::hal::audio::{AudioSource, MockMic, vad_energy};
let mut mic = MockMic::new();
let mut buf = [0i16; 128];
let n = mic.read(&mut buf);
if vad_energy(&buf[..n]) { /* voice detected */ }
```
VAD: RMS over `buf`, threshold `VAD_THRESHOLD = 2048` (Q15-ish).
No new deps — square wave avoids `libm`/`micromath`.
## GPIO
`hal::gpio` — runtime digital R/W via `Flex` + `AnyPin::steal`.
Reserved pins (runtime guard): 0, 3, 26-32 (flash/PSRAM), 45, 46 (strapping), 48 (LED).
```rust
use esp_s3_hal::hal::gpio;
gpio::write_pin(5, gpio::PinAction::High)?;   // set GPIO5 high
let level = gpio::read_pin(5)?;                 // read GPIO5
```
## PWM
`hal::pwm` — LEDC wrapper. 8 low-speed channels (0-7), any output GPIO.
```rust
use esp_s3_hal::hal::pwm;
pwm::set_pwm(5, 0, 1000, 50)?;    // GPIO5, channel 0, 1kHz, 50% duty
pwm::set_duty(0, 75)?;            // update duty to 75% (register-direct)
pwm::stop_pwm(0)?;                // stop channel (duty 0)
```
## ADC
`hal::adc` — ADC1 only (ADC2 conflicts with Wi-Fi). GPIO1,2,4-10.
```rust
use esp_s3_hal::hal::adc;
let mv = adc::read_mv(1)?;           // single sample, returns millivolts
let mv = adc::read_mv_avg(1, 8)?;    // 8-sample average
```
## I2C
`hal::i2c` — async master, 7-bit addressing. Configure bus once, then R/W.
Requires pull-up resistors (2.2k-10k) on SDA/SCL. 500ms async timeout.
```rust
use esp_s3_hal::hal::i2c;
i2c::configure_bus(8, 9, 100)?;              // SDA=GPIO8, SCL=GPIO9, 100kHz
i2c::write_reg(0x3c, 0x00, &[1, 2, 3]).await?;  // addr=0x3c, reg=0x00
let data = i2c::read_reg(0x3c, 0x00, 6).await?;
```
## BLE
`ble` — esp-radio HCI connector (BLE 5.0, coex with Wi-Fi).
```rust
use esp_s3_hal::ble;
let connector = ble::init()?;  // BleConnector for HCI read/write
```
GATT services + audio streaming are built on top in the firmware.
