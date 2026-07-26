# esp-s3-hal
ESP32-S3-WROOM-1 · Reusable Rust `no_std` HAL: WS2812 LED, GPIO, PWM, ADC, I2C, BLE, Wi-Fi (AP/STA), HTTP building blocks.
**WTFPL-2.0**
## Tree
```
src/
  boot/brownout.rs   RTC BOD disable (Wi-Fi TX spike safety)
  hal/led.rs         Led · LedState · LedCommand (ws2812-rmt, GPIO48)
  hal/gpio.rs        runtime digital R/W (Flex + AnyPin::steal, reserved-pin guard)
  hal/pwm.rs         LEDC wrapper (8 channels, set_duty via register-direct)
  hal/adc.rs         ADC1 reader (GPIO1,2,4-10, multi-sample avg)
  hal/i2c.rs         async I2C master (configure_bus, read_reg, write_reg, 500ms timeout)
  hal/audio.rs       AudioSource trait + MockMic + vad_energy
  ble.rs             esp-radio HCI connector (BLE 5.0, coex with Wi-Fi)
  net.rs             init_ap(ssid,pass,ip,gw,subnet) · init_sta(ssid,pass) · net_task · dhcp_task
  http.rs            LedMutex type alias
examples/            basic_led · wifi_scan · http_led
docs/                architecture · hal · net · http · boot · flash · examples · ci
scripts/             flash.sh · monitor.py
```
Folder = domain. Open the folder; stop reading filenames as a bag.
## Use
Add to your `Cargo.toml`:
```toml
esp-s3-hal = { path = "../esp-s3-hal" }   # or git = "https://github.com/oblx/esp-s3-hal"
```
```rust
use esp_s3_hal::hal::led::{Led, LedCommand};
use esp_s3_hal::net;
use esp_s3_hal::boot::brownout;

let mut led = Led::new(rmt, peripherals.GPIO48);
led.apply(LedCommand { state: "on".into(), r: 255, g: 0, b: 0, intensity: 0.8 }).await;
```
## Run examples
```bash
source ~/export-esp.sh
cargo +esp build --release --example basic_led -Zbuild-std=core,compiler_builtins,alloc
espflash flash --min-chip-rev 0.0 target/xtensa-esp32s3-none-elf/release/examples/basic_led
```
## Surface
| module | API | purpose |
|-|-|-|
| `boot::brownout` | `disable()` | RTC BOD off (Wi-Fi TX spikes) |
| `hal::led` | `Led::new(rmt, pin)` · `apply(cmd)` | WS2812 driver abstraction |
| `hal::led` | `LedState` · `LedCommand` | shared state + JSON command types |
| `hal::gpio` | `read_pin` · `write_pin` · `reset_all` | runtime digital R/W (Flex + steal) |
| `hal::pwm` | `set_pwm` · `set_duty` · `stop_pwm` | LEDC PWM (8 channels) |
| `hal::adc` | `read_mv` · `read_mv_avg` | ADC1 reader (millivolts) |
| `hal::i2c` | `configure_bus` · `read_reg` · `write_reg` | async I2C master (500ms timeout) |
| `hal::audio` | `AudioSource` · `MockMic` · `vad_energy` | audio capture + VAD (mock Phase-1) |
| `ble` | `init()` · `is_enabled()` | BLE HCI connector (coex with Wi-Fi) |
| `net` | `init_ap(…)` · `init_sta(…)` | Wi-Fi AP/STA init (parameterized) |
| `net` | `net_task` · `dhcp_task` | embassy tasks for net stack |
| `http` | `LedMutex` | shared LED state mutex type |
## Facts
- Target: `xtensa-esp32s3-none-elf` · build-std `core,compiler_builtins,alloc`
- LED driver: `ws2812-rmt` 0.2.0 (not `esp-hal-smartled2` — timing bug)
- LED pin: GPIO48 · Flash: `--min-chip-rev 0.0`
- `mem::forget(ctrl)` — WifiController Drop deinitializes radio
- No env consts — all functions take parameters (decoupled from app config)
## Docs map
| file | SSOT for |
|-|-|
| `docs/architecture.md` | layers + dependency direction |
| `docs/hal.md` | WS2812 · GPIO · PWM · ADC · I2C · BLE · audio · brownout |
| `docs/net.md` | init_ap/init_sta · net_task · dhcp_task |
| `docs/http.md` | LedMutex type · usage pattern |
| `docs/boot.md` | brownout disable |
| `docs/flash.md` | espflash · boot mode · min chip rev |
| `docs/examples.md` | what each example proves |
| `docs/ci.md` | Gitea Actions · GitHub mirror |
| `AGENTS.md` | agent verify invariants |
## CI
Gitea source of truth · mirror to public GitHub on `main` · ship tags `ship/esp-s3-hal-*`. Zero hardcode in YAML — see `docs/ci.md`.
