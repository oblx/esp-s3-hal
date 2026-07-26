# Architecture
```
examples/*/main          consumer crate
        │
        ▼
   esp_s3_hal
   ├── boot::brownout    disable()
   ├── hal::led          Led · LedState · LedCommand
   ├── hal::gpio         read_pin · write_pin · reset_all (Flex + steal)
   ├── hal::pwm          set_pwm · set_duty · stop_pwm (LEDC)
   ├── hal::adc          read_mv · read_mv_avg (ADC1)
   ├── hal::i2c          configure_bus · read_reg · write_reg (async)
   ├── hal::audio        AudioSource · MockMic · vad_energy
   ├── ble               init() · is_enabled() (HCI connector)
   ├── net               init_ap · init_sta · net_task · dhcp_task
   └── http              LedMutex type
        │
        ▼
   esp-hal · esp-radio · embassy · picoserve · ws2812-rmt · nb
```
## Layers
| layer | crate | role |
|-|-|-|
| runtime | `esp-rtos` + `embassy-executor` | async tasks, timers |
| hal | `esp-hal` + `ws2812-rmt` + `nb` | peripherals, LED, ADC blocking |
| net | `esp-radio` + `embassy-net` | Wi-Fi/BLE + IP stack |
| web | `picoserve` + `miniserde` | HTTP server + JSON |
| sdk | `esp_s3_hal` | reusable abstractions over above |
## Dependency direction
`examples` → `esp_s3_hal` → `esp-hal` ecosystem
`hal::*` modules are leaves (no internal deps, steal-based runtime access).
`ble` → `esp-radio::ble` (HCI connector, coex with Wi-Fi).
`net` is a leaf (no internal deps, params only).
`http` → `hal::led` (LedState type).
No upward imports. No env consts.
