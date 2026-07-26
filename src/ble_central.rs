//! BLE central role — scan, connect, GATT client via trouble-host 0.6.
//!
//! Wraps the HCI connector into a trouble-host stack with central role.
//! Scan results arrive via EventHandler callback (trouble-host architecture).
//! The firmware spawns `ble_central_task` which runs the host runner loop.
//! HTTP handlers call scan/connect/read/write via `BleCentralHandle`.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use esp_radio::ble::controller::BleConnector;
use heapless::Deque;
use trouble_host::prelude::*;

const CONN_MAX: usize = 1;
const L2CAP_MAX: usize = 1;
const SCAN_CACHE: usize = 32;

/// A discovered device from a scan.
#[derive(Clone)]
pub struct ScanResult {
	pub addr: [u8; 6],
	pub addr_type: u8,
	pub name: String,
	pub rssi: i8,
}

/// Commands sent from HTTP handlers to the BLE task.
pub enum BleCmd {
	Scan,
	Connect([u8; 6], u8),
	Disconnect,
	DiscoverServices,
	ReadCharacteristic(u16),
	WriteCharacteristic(u16, Vec<u8>),
}

/// Responses from the BLE task back to HTTP handlers.
pub enum BleResp {
	ScanResults(Vec<ScanResult>),
	Connected,
	Disconnected,
	Services(Vec<String>),
	ReadData(Vec<u8>),
	Written,
	Error(String),
}

/// Handle for HTTP handlers to interact with BLE central.
pub struct BleCentralHandle {
	pub cmd: Channel<CriticalSectionRawMutex, BleCmd, 4>,
	pub resp: Channel<CriticalSectionRawMutex, BleResp, 4>,
}

impl BleCentralHandle {
	pub const fn new() -> Self {
		Self {
			cmd: Channel::new(),
			resp: Channel::new(),
		}
	}

	pub async fn scan(&self) -> Result<Vec<ScanResult>, String> {
		self.cmd.send(BleCmd::Scan).await;
		match self.resp.receive().await {
			BleResp::ScanResults(r) => Ok(r),
			BleResp::Error(e) => Err(e),
			_ => Err("unexpected response".into()),
		}
	}

	pub async fn connect(&self, addr: [u8; 6], t: u8) -> Result<(), String> {
		self.cmd.send(BleCmd::Connect(addr, t)).await;
		match self.resp.receive().await {
			BleResp::Connected => Ok(()),
			BleResp::Error(e) => Err(e),
			_ => Err("unexpected response".into()),
		}
	}

	pub async fn disconnect(&self) -> Result<(), String> {
		self.cmd.send(BleCmd::Disconnect).await;
		match self.resp.receive().await {
			BleResp::Disconnected => Ok(()),
			BleResp::Error(e) => Err(e),
			_ => Err("unexpected response".into()),
		}
	}

	pub async fn discover_services(&self) -> Result<Vec<String>, String> {
		self.cmd.send(BleCmd::DiscoverServices).await;
		match self.resp.receive().await {
			BleResp::Services(s) => Ok(s),
			BleResp::Error(e) => Err(e),
			_ => Err("unexpected response".into()),
		}
	}

	pub async fn read(&self, h: u16) -> Result<Vec<u8>, String> {
		self.cmd.send(BleCmd::ReadCharacteristic(h)).await;
		match self.resp.receive().await {
			BleResp::ReadData(d) => Ok(d),
			BleResp::Error(e) => Err(e),
			_ => Err("unexpected response".into()),
		}
	}

	pub async fn write(&self, h: u16, d: Vec<u8>) -> Result<(), String> {
		self.cmd.send(BleCmd::WriteCharacteristic(h, d)).await;
		match self.resp.receive().await {
			BleResp::Written => Ok(()),
			BleResp::Error(e) => Err(e),
			_ => Err("unexpected response".into()),
		}
	}
}

/// Event handler that collects scan results.
struct ScanCollector {
	seen: RefCell<Deque<BdAddr, SCAN_CACHE>>,
	results: RefCell<Vec<ScanResult>>,
}

impl ScanCollector {
	fn new() -> Self {
		Self {
			seen: RefCell::new(Deque::new()),
			results: RefCell::new(Vec::new()),
		}
	}

	fn drain(&self) -> Vec<ScanResult> {
		let mut r = self.results.borrow_mut();
		let out = r.clone();
		r.clear();
		self.seen.borrow_mut().clear();
		out
	}
}

impl EventHandler for ScanCollector {
	fn on_adv_reports(&self, mut it: LeAdvReportsIter<'_>) {
		let mut seen = self.seen.borrow_mut();
		let mut results = self.results.borrow_mut();
		while let Some(Ok(report)) = it.next() {
			let addr_bytes: [u8; 6] = {
				let r = report.addr.raw();
				[r[0], r[1], r[2], r[3], r[4], r[5]]
			};
			if seen.iter().any(|b| {
				let br = b.raw();
				br == addr_bytes
			}) {
				continue;
			}
			let mut name = String::new();
			for ad in AdStructure::decode(report.data) {
				if let Ok(AdStructure::CompleteLocalName(n)) = ad {
					name = String::from_utf8_lossy(n).into();
				}
			}
			results.push(ScanResult {
				addr: addr_bytes,
				addr_type: report.addr_kind.as_raw(),
				name,
				rssi: report.rssi,
			});
			if seen.is_full() {
				seen.pop_front();
			}
			let _ = seen.push_back(BdAddr::new(addr_bytes));
			if results.len() >= 32 {
				break;
			}
		}
	}
}

type Ctl = ExternalController<BleConnector<'static>, 20>;

/// Run the BLE central task. Spawn this on the main executor (CPU0).
/// The future is boxed to keep the state machine on the heap.
/// Requires the main stack to be large enough for HCI event polling
/// (~30KB+); use `#[ram(reclaimed)]` for the heap to maximize stack.
#[embassy_executor::task(pool_size = 1)]
pub async fn ble_central_task(
	connector: BleConnector<'static>,
	handle: &'static BleCentralHandle,
) {
	Box::pin(ble_central_inner(connector, handle)).await
}

async fn ble_central_inner(
	connector: BleConnector<'static>,
	handle: &'static BleCentralHandle,
) {
	let controller = ExternalController::<_, 20>::new(connector);
	let mut resources: HostResources<DefaultPacketPool, CONN_MAX, L2CAP_MAX> =
		HostResources::new();
	let stack = trouble_host::new(controller, &mut resources)
		.set_random_address(Address::random([0xff, 0x8f, 0x1b, 0x05, 0xe4, 0xff]));
	let host = stack.build();
	let collector = ScanCollector::new();
	let mut runner = host.runner;
	let mut scanner = Scanner::new(host.central);
	let _ = embassy_futures::join::join(
		runner.run_with_handler(&collector),
		command_loop(scanner, &collector, handle),
	)
	.await;
}

async fn command_loop(
	mut scanner: Scanner<'_, Ctl, DefaultPacketPool>,
	collector: &ScanCollector,
	handle: &'static BleCentralHandle,
) {
	loop {
		let cmd = handle.cmd.receive().await;
		match cmd {
			BleCmd::Scan => {
				let config = ScanConfig::default();
				match scanner.scan(&config).await {
					Ok(_session) => {
						embassy_time::Timer::after(embassy_time::Duration::from_secs(3)).await;
						let results = collector.drain();
						let _ = handle.resp.send(BleResp::ScanResults(results)).await;
					}
					Err(e) => {
						let _ = handle.resp.send(BleResp::Error(format!("scan: {:?}", e))).await;
					}
				}
			}
			BleCmd::Connect(addr, _t) => {
				let mut central = scanner.into_inner();
				let target = Address::random(addr);
				let config = ConnectConfig {
					connect_params: Default::default(),
					scan_config: ScanConfig {
						filter_accept_list: &[(AddrKind::RANDOM, &target.addr)],
						..Default::default()
					},
				};
				match central.connect(&config).await {
					Ok(conn) => {
						let _ = handle.resp.send(BleResp::Connected).await;
						loop {
							if let BleCmd::Disconnect = handle.cmd.receive().await {
								conn.disconnect();
								let _ = handle.resp.send(BleResp::Disconnected).await;
								break;
							}
						}
					}
					Err(e) => {
						let _ = handle.resp.send(BleResp::Error(format!("connect: {:?}", e))).await;
					}
				}
				scanner = Scanner::new(central);
			}
			BleCmd::Disconnect => {
				let _ = handle.resp.send(BleResp::Error("not connected".into())).await;
			}
			BleCmd::DiscoverServices => {
				let _ = handle.resp.send(BleResp::Error("not connected".into())).await;
			}
			BleCmd::ReadCharacteristic(_) => {
				let _ = handle.resp.send(BleResp::Error("not connected".into())).await;
			}
			BleCmd::WriteCharacteristic(_, _) => {
				let _ = handle.resp.send(BleResp::Error("not connected".into())).await;
			}
		}
	}
}
