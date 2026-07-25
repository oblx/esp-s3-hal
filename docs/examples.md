# Examples
Standalone binaries in `examples/`. Each demonstrates SDK APIs.
```bash
cargo +esp build --release --example <name> -Zbuild-std=core,compiler_builtins,alloc
espflash flash --min-chip-rev 0.0 target/xtensa-esp32s3-none-elf/release/examples/<name>
```
| name | Wi-Fi | HTTP | SDK APIs used | proves |
|-|-|-|-|-|
| `basic_led` | no | no | `hal::led::Led`, `LedCommand` | LED color cycle via SDK abstraction |
| `wifi_scan` | yes | no | `esp_radio` scan | AP scanning + RSSI print |
| `http_led` | yes | yes | `hal::led`, `net::init_sta`, `net::net_task` | full stack: Wi-Fi + HTTP + LED |
`wifi_scan` uses raw `esp_radio` scan API (no SDK scan abstraction yet).
`http_led` demonstrates the SDK pattern: `init_sta` + `net_task` + custom HTTP task.
