#![no_std]
#![feature(impl_trait_in_assoc_type)]

extern crate alloc;

pub mod boot;
pub mod hal;

#[cfg(any(feature = "wifi-ap", feature = "wifi-sta"))]
pub mod ble;

#[cfg(any(feature = "wifi-ap", feature = "wifi-sta"))]
pub mod net;

#[cfg(any(feature = "wifi-ap", feature = "wifi-sta"))]
pub mod http;
