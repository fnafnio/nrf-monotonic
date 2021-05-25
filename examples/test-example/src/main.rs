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

use defmt_rtt as _;
use panic_probe as _;
// #[cfg(feature = "52840")]
use nrf52840_hal as hal;

static COUNT: AtomicUsize = AtomicUsize::new(0);
defmt::timestamp!("{=usize}", COUNT.fetch_add(1, Ordering::Relaxed));

// use

#[app(device = crate::hal::pac, dispatchers = [PWM0])]
mod APP {
    use nrf_monotonic::NrfMonotonic;
    use rtic::time::duration::Seconds;

    #[monotonic(binds = TIMER0, default = true)]
    type MyMono = NrfMonotonic<crate::hal::pac::TIMER0>;

    #[init]
    fn init(cx: init::Context) -> (init::LateResources, init::Monotonics) {
        defmt::info!("init");

        let mono = NrfMonotonic::new(cx.device.TIMER0);

        tttask::spawn_after(Seconds(1_u32)).unwrap();

        (init::LateResources {}, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        defmt::info!("idle");

        // cortex_m::asm::bkpt();

        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task]
    fn tttask(_: tttask::Context) {
        blachz::spawn_after(Seconds(1_u32)).unwrap();
        defmt::info!("TTTask");
    }

    #[task]
    fn blachz(_: blachz::Context) {
        defmt::info!("Blachz");
        tttask::spawn_after(Seconds(1_u32)).unwrap();
    }
}
