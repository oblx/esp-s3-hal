# Net
Wi-Fi init via `esp-radio` + `embassy-net`. All functions take **parameters** — no env consts.
## AP mode
```rust
use esp_s3_hal::net;

let (dev, cfg) = net::init_ap("my-ssid", "password", "192.168.2.1", "192.168.2.1", 24);
let (stack, runner) = embassy_net::new(dev, cfg, resources, seed);
spawner.spawn(net::net_task(runner).unwrap());
spawner.spawn(net::dhcp_task(stack).unwrap());
```
## STA mode
```rust
let (dev, cfg, mut ctrl) = net::init_sta("ssid", "password");
let (stack, runner) = embassy_net::new(dev, cfg, resources, seed);
spawner.spawn(net::net_task(runner).unwrap());
ctrl.connect_async().await.expect("connect");
core::mem::forget(ctrl);  // Drop kills radio
```
## Tasks
| task | role |
|-|-|
| `net_task(runner)` | drives embassy-net stack |
| `dhcp_task(stack)` | AP mode DHCP server (192.168.2.50-200) |
## Invariant
`mem::forget(ctrl)` — `WifiController::drop` deinitializes the radio.
Must forget after `connect_async()` or DHCP never completes.
