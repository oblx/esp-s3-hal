use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::hal::led::LedState;

pub type LedMutex = Mutex<CriticalSectionRawMutex, LedState>;
