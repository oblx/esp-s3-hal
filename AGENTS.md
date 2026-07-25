# Agents
```bash
source ~/export-esp.sh
cargo +esp build --release -Zbuild-std=core,compiler_builtins,alloc
cargo +esp build --release --examples -Zbuild-std=core,compiler_builtins,alloc
cargo +esp build --release --no-default-features --features wifi-sta --examples -Zbuild-std=core,compiler_builtins,alloc
scripts/flash.sh ap basic_led
python3 scripts/monitor.py
```
| invariant | |
|-|-|
| target | `xtensa-esp32s3-none-elf` only |
| build-std | `core,compiler_builtins,alloc` (no prebuilt core for xtensa) |
| wifi feature | exactly one of `wifi-ap` \| `wifi-sta` (compile_error guard) |
| led driver | `ws2812-rmt` 0.2.0 (not `esp-hal-smartled2` — timing bug) |
| led pin | GPIO48 |
| wifi ctrl | `mem::forget(ctrl)` — Drop deinitializes radio, kills DHCP |
| no env consts | all public functions take parameters (decoupled from app config) |
| layout | folder = domain · docs named by domain · no numbered dump folders |
| deps | crates.io allowed (esp-hal ecosystem) |
| ci yaml | vars+secrets only · no hostnames |
| mirror | `GH_DEPLOY_KEY` · strip `.gitea` before github push |
Future: BLE · I2S mic · e-paper · OTA partitions.
