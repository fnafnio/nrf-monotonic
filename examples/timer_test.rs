#![no_main]
#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};

use rtic::app;

#[cfg(feature = "51")]
use nrf51_hal as hal;

#[cfg(feature = "52810")]
use nrf52810_hal as hal;

#[cfg(feature = "52811")]
use nrf52811_hal as hal;

#[cfg(feature = "52832")]
use nrf52832_hal as hal;

use panic_probe as _;
use defmt_rtt as _;
// #[cfg(feature = "52840")]
use nrf52840_hal as hal;


static COUNT: AtomicUsize = AtomicUsize::new(0);
defmt::timestamp!("{=usize}", COUNT.fetch_add(1, Ordering::Relaxed));

#[app(device = crate::hal::pac)]
mod APP {
    #[init]
    fn init(_: init::Context) -> (init::LateResources, init::Monotonics) {
        defmt::info!("init");
        // hprintln!("init").unwrap();
        (init::LateResources{}, init::Monotonics{})
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        defmt::info!("idle");

        cortex_m::asm::bkpt();

        loop {
            continue;
        }
    }
}