#[cfg(feature = "wifi-ap")]
use core::net::Ipv4Addr;
#[cfg(feature = "wifi-ap")]
use core::str::FromStr;

use embassy_net::Config;
#[cfg(feature = "wifi-ap")]
use embassy_net::{Ipv4Cidr, Stack, StaticConfigV4};
#[cfg(feature = "wifi-ap")]
use embassy_time::Duration;
use esp_println::println;
#[cfg(feature = "wifi-ap")]
use esp_hal_dhcp_server::{run_dhcp_server, simple_leaser::SimpleDhcpLeaser, structs::DhcpServerConfig};
#[cfg(feature = "wifi-ap")]
use esp_radio::wifi::ap::AccessPointConfig;
use esp_radio::wifi::{Config as WifiConfig, ControllerConfig, Interface};
#[cfg(feature = "wifi-sta")]
use esp_radio::wifi::sta::StationConfig;

pub type WifiDevice = Interface;

#[cfg(feature = "wifi-ap")]
pub fn init_ap(
	ssid: &str,
	pass: &str,
	ip: &str,
	gateway: &str,
	subnet: u8,
) -> (WifiDevice, Config) {
	let ap_config = AccessPointConfig::default()
		.with_ssid(ssid)
		.with_password(pass.into())
		.with_auth_method(esp_radio::wifi::AuthenticationMethod::Wpa2Personal);
	let ctrl_cfg = ControllerConfig::default()
		.with_initial_config(WifiConfig::AccessPoint(ap_config));
	let ctrl = esp_radio::wifi::WifiController::new(
		unsafe { esp_hal::peripherals::WIFI::steal() }, ctrl_cfg,
	).unwrap();
	core::mem::forget(ctrl);
	println!("Wi-Fi AP started: SSID={}", ssid);
	let dev = Interface::access_point();
	let cfg = Config::ipv4_static(StaticConfigV4 {
		address: Ipv4Cidr::new(
			Ipv4Addr::from_str(ip).unwrap(),
			subnet,
		),
		gateway: Some(Ipv4Addr::from_str(gateway).unwrap()),
		dns_servers: Default::default(),
	});
	(dev, cfg)
}

#[cfg(feature = "wifi-sta")]
pub fn init_sta(
	ssid: &str,
	pass: &str,
) -> (WifiDevice, Config, esp_radio::wifi::WifiController<'static>) {
	let sta_config = StationConfig::default()
		.with_ssid(ssid)
		.with_password(pass.into());
	let ctrl_cfg = ControllerConfig::default()
		.with_initial_config(WifiConfig::Station(sta_config));
	let ctrl = esp_radio::wifi::WifiController::new(
		unsafe { esp_hal::peripherals::WIFI::steal() }, ctrl_cfg,
	).unwrap();
	println!("Wi-Fi STA init: SSID={}", ssid);
	let dev = Interface::station();
	let cfg = Config::dhcpv4(embassy_net::DhcpConfig::default());
	(dev, cfg, ctrl)
}

#[embassy_executor::task]
pub async fn net_task(mut runner: embassy_net::Runner<'static, Interface>) {
	runner.run().await
}

#[embassy_executor::task]
#[cfg(feature = "wifi-ap")]
pub async fn dhcp_task(stack: Stack<'static>) {
	let config = DhcpServerConfig {
		ip: Ipv4Addr::new(192, 168, 2, 1),
		lease_time: Duration::from_secs(3600),
		gateways: &[],
		subnet: None,
		dns: &[],
		use_captive_portal: false,
	};
	let mut leaser = SimpleDhcpLeaser {
		start: Ipv4Addr::new(192, 168, 2, 50),
		end: Ipv4Addr::new(192, 168, 2, 200),
		leases: Default::default(),
	};
	let _ = run_dhcp_server(stack, config, &mut leaser).await;
}
