# HTTP
The SDK provides shared types for HTTP + LED state. The app crate owns routes, handlers, and the web task.
## LedMutex
```rust
use esp_s3_hal::http::LedMutex;
use esp_s3_hal::hal::led::LedState;

static LED: StaticCell<LedMutex> = StaticCell::new();
let led = LED.init(LedMutex::new(LedState::default()));
```
`LedMutex = Mutex<CriticalSectionRawMutex, LedState>`
## Usage pattern
The app crate defines `AppState`, `AppProps`, routes, and `web_task`:
```rust
struct AppState { sender: Sender<…>, led: &'static LedMutex }
impl AppWithStateBuilder for AppProps { … }
spawner.spawn(web_task(0, stack, AppProps, cfg, state, port).unwrap());
```
The SDK does not provide `web_task` — `#[embassy_executor::task]` can't be generic over app state. Each app wraps `picoserve::Server` in its own task.
## Why not generic web_task
`embassy_executor::task` macro doesn't support generic type params. The 6-line picoserve listen loop is app-specific (state type, buffer sizes). DRY here would add complexity without benefit.
