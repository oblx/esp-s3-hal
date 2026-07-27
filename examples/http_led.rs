//! HTTP LED control example — standalone HTTP server + WS2812 via esp_s3_hal.
//!
//! Run: `cargo +esp build --release --no-default-features --features wifi-sta --example http_led -Zbuild-std=core,compiler_builtins,alloc`
//! Set STA_SSID / STA_PASS env vars, flash, then:
//!   curl -X POST http://<ip>/on     # LED red
//!   curl -X POST http://<ip>/off    # LED off

#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

esp_bootloader_esp_idf::esp_app_desc!();

use embassy_executor::Spawner;
use embassy_net::{Stack, StackResources};
use esp_backtrace as _;
use esp_hal::interrupt::software::SoftwareInterruptControl;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use picoserve::{AppWithStateBuilder, response::fs::File, routing::{get_service, post}};
use static_cell::StaticCell;

use esp_s3_hal::hal::led::{Led, LedCommand, LedState};
use esp_s3_hal::http::LedMutex;
use esp_s3_hal::net;

const STA_SSID: &str = match option_env!("STA_SSID") { Some(v) => v, None => "" };
const STA_PASS: &str = match option_env!("STA_PASS") { Some(v) => v, None => "" };

static STACK_RES: StaticCell<StackResources<3>> = StaticCell::new();
static PICO_CFG: StaticCell<picoserve::Config> = StaticCell::new();
static LED_MX: StaticCell<LedMutex> = StaticCell::new();

struct AppProps;

impl AppWithStateBuilder for AppProps {
	type State = &'static LedMutex;
	type PathRouter = impl picoserve::routing::PathRouter<Self::State>;
	fn build_app(self) -> picoserve::Router<Self::PathRouter, Self::State> {
		picoserve::Router::new()
			.route("/", get_service(File::html("<h1>http_led</h1><a href='/on'>ON</a> <a href='/off'>OFF</a>")))
			.route("/on", post(|picoserve::extract::State(l): picoserve::extract::State<&'static LedMutex>| async move {
				{ let mut s = l.lock().await; s.on = true; s.r = 255; s.g = 0; s.b = 0; s.level = 204; }
				"on"
			}))
			.route("/off", post(|picoserve::extract::State(l): picoserve::extract::State<&'static LedMutex>| async move {
				{ let mut s = l.lock().await; s.on = false; s.r = 0; s.g = 0; s.b = 0; s.level = 0; }
				"off"
			}))
	}
}

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
	println!("http_led start");
	let peripherals = esp_hal::init(esp_hal::Config::default());
	esp_alloc::heap_allocator!(size: 128 * 1024);

	let timg0 = TimerGroup::new(peripherals.TIMG0);
	let sw_int = SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
	esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);

	let (dev, cfg, mut ctrl) = net::init_sta(STA_SSID, STA_PASS, "esp32-s3");
	let (stack, runner) = embassy_net::new(
		dev, cfg, STACK_RES.init(StackResources::<3>::new()), 0x1234_5678_9abc_def0,
	);
	spawner.spawn(net::net_task(runner).unwrap());
	println!("connecting to {}...", STA_SSID);
	ctrl.connect_async().await.expect("wifi connect");
	core::mem::forget(ctrl);
	println!("connected");

	let rmt = esp_hal::rmt::Rmt::new(peripherals.RMT, esp_hal::time::Rate::from_mhz(80)).unwrap();
	let led = Led::new(rmt, peripherals.GPIO48);
	let led_mx = LED_MX.init(LedMutex::new(LedState::default()));

	spawner.spawn(led_task(led, led_mx).unwrap());
	let pcfg = PICO_CFG.init(picoserve::Config::new(picoserve::Timeouts::const_default()));
	spawner.spawn(web_task(0, stack, AppProps, pcfg, led_mx, 80).unwrap());

	loop {
		if let Some(c) = stack.config_v4() {
			println!("IP: {}", c.address);
			loop { embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await; }
		}
		embassy_time::Timer::after(embassy_time::Duration::from_millis(500)).await;
	}
}

#[embassy_executor::task]
async fn led_task(mut led: Led, mx: &'static LedMutex) {
	loop {
		let s = mx.lock().await;
		let cmd = LedCommand {
			state: if s.on { "on" } else { "off" }.into(),
			r: s.r, g: s.g, b: s.b,
			intensity: s.level as f64 / 255.0,
		};
		led.apply(cmd);
		embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
	}
}

#[embassy_executor::task]
async fn web_task(id: usize, stack: Stack<'static>, props: AppProps, cfg: &'static picoserve::Config, mx: &'static LedMutex, port: u16) {
	let mut rx = [0u8; 1024]; let mut tx = [0u8; 1024]; let mut buf = [0u8; 2048];
	let app = props.build_app().with_state(mx);
	loop {
		picoserve::Server::new(&app, cfg, &mut buf).listen_and_serve(id, stack, port, &mut rx, &mut tx).await;
	}
}
