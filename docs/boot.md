# Boot
## Brownout disable
```rust
use esp_s3_hal::boot::brownout;

brownout::disable();
```
Clears `RTC_CNTL.brown_out` RST + ENA bits.
Wi-Fi TX draws 300-500mA spikes. On weak USB power, the BOD triggers a reset (`rst:0xf`). Disabling is a safety net — use a stable power supply (powered hub, USB 3.0, or dual USB-C cables).
## Boot sequence (app crate responsibility)
```
ROM → app_desc → esp_hal::init → brownout::disable → heap_allocator
    → esp_rtos::start → init_wifi → Led::new → spawn tasks → idle
```
The SDK provides `brownout::disable()`. The app crate owns the rest of the boot sequence.
