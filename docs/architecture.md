# Architecture
```
examples/*/main          consumer crate
        │
        ▼
   esp_s3_hal
   ├── boot::brownout    disable()
   ├── hal::led          Led · LedState · LedCommand
   ├── net               init_ap · init_sta · net_task · dhcp_task
   └── http              LedMutex type
        │
        ▼
   esp-hal · esp-radio · embassy · picoserve · ws2812-rmt
```
## Layers
| layer | crate | role |
|-|-|-|
| runtime | `esp-rtos` + `embassy-executor` | async tasks, timers |
| hal | `esp-hal` + `ws2812-rmt` | peripherals, LED |
| net | `esp-radio` + `embassy-net` | Wi-Fi + IP stack |
| web | `picoserve` + `miniserde` | HTTP server + JSON |
| sdk | `esp_s3_hal` | reusable abstractions over above |
## Dependency direction
`examples` → `esp_s3_hal` → `esp-hal` ecosystem
`hal::led` is a leaf (no internal deps).
`net` is a leaf (no internal deps, params only).
`http` → `hal::led` (LedState type).
No upward imports. No env consts.
