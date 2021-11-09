//! # `Monotonic` implementation based on DWT and SysTick

#![no_std]
#![allow(unused_macros)]

#[cfg(feature = "51")]
use nrf51_hal as hal;

#[cfg(feature = "52810")]
use nrf52810_hal as hal;

#[cfg(feature = "52811")]
use nrf52811_hal as hal;

#[cfg(feature = "52832")]
use nrf52832_hal as hal;

#[cfg(feature = "52840")]
use nrf52840_hal as hal;

mod fmt_helpers;

mod timer_monotonic;
pub use timer_monotonic::NrfMonotonic;

// mod rtc_monotonic;
// pub use rtc_monotonic::RtcMonotonic;

// mod rtc_monotonic_v2;
// pub use rtc_monotonic_v2::RtcMono;
