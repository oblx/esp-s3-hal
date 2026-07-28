# HTTP
The SDK provides shared types for HTTP + LED state + CORS. The app crate owns routes, handlers, and the web task.
## LedMutex
```rust
use esp_s3_hal::http::LedMutex;
use esp_s3_hal::hal::led::LedState;

static LED: StaticCell<LedMutex> = StaticCell::new();
let led = LED.init(LedMutex::new(LedState::default()));
```
`LedMutex = Mutex<CriticalSectionRawMutex, LedState>`
## CORS types
`CorsResponse<T>` — wrapper that adds `Access-Control-Allow-Origin: *` + methods + headers headers to any picoserve response. Usage: `async fn handler() -> CorsResponse<String> { CorsResponse(json) }`
`CorsPreflight` — empty 204 response with CORS headers for OPTIONS preflight. Usage: `.options(cors_preflight)` where `async fn cors_preflight() -> CorsPreflight { CorsPreflight }`
Both gated behind `wifi-ap` or `wifi-sta` feature.
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
