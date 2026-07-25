# Flash
Tool: `espflash` (Rust). Port: `/dev/ttyACM0`.
## Build
```bash
source ~/export-esp.sh
cargo +esp build --release -Zbuild-std=core,compiler_builtins,alloc
```
AP (default) or STA:
```bash
cargo +esp build --release --no-default-features --features wifi-sta \
	-Zbuild-std=core,compiler_builtins,alloc
```
## Flash an example
```bash
cargo +esp build --release --example basic_led -Zbuild-std=core,compiler_builtins,alloc
espflash flash --min-chip-rev 0.0 --port /dev/ttyACM0 \
	target/xtensa-esp32s3-none-elf/release/examples/basic_led
```
Or: `scripts/flash.sh ap basic_led`
## Monitor
```bash
espflash monitor --port /dev/ttyACM0
python3 scripts/monitor.py
```
## Boot mode
| GPIO0 during RST | mode | use |
|-|-|-|
| LOW | download (boot:0x0) | flash |
| HIGH | normal (boot:0x8) | run |
## Min chip rev
Board reports efuse v1.4 but chip v0.2. Flash with `--min-chip-rev 0.0`.
