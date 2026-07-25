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
