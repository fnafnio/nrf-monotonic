#![no_main]
#![no_std]

use rtic::app;

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

use defmt_rtt as _;
use panic_probe as _;

defmt::timestamp!("{=u64:µs}", { get_time_from_rtic() });

fn get_time_from_rtic() -> u64 {
    use rtic::rtic_monotonic::Instant;
    let t: Instant<_> = app::monotonics::now();
    *t.duration_since_epoch().integer()
}

#[app(device = crate::hal::pac, dispatchers = [PWM0])]
mod app {
    use crate::hal;
    use hal::clocks;
    use hal::gpio::{Level, Output, Pin, PushPull};
    use hal::prelude::{OutputPin, StatefulOutputPin};
    use nrf_monotonic::NrfMonotonic;
    use rtic::time::duration::{Milliseconds, Seconds};

    #[monotonic(binds = TIMER0, default = true)]
    type MyMono = NrfMonotonic<crate::hal::pac::TIMER0>;

    #[resources]
    struct Resources {
        #[task_local]
        led: Pin<Output<PushPull>>,
        #[task_local]
        pin: Pin<Output<PushPull>>,
    }

    #[init]
    fn init(cx: init::Context) -> (init::LateResources, init::Monotonics) {
        clocks::Clocks::new(unsafe { core::mem::transmute(()) })
            .enable_ext_hfosc()
            .set_lfclk_src_external(clocks::LfOscConfiguration::NoExternalNoBypass)
            .start_lfclk();

        let mono = NrfMonotonic::new(cx.device.TIMER0);

        let port0 = hal::gpio::p0::Parts::new(cx.device.P0);
        let led = port0.p0_13.into_push_pull_output(Level::High).degrade();
        let pin = port0.p0_31.into_push_pull_output(Level::High).degrade();

        // defmt::info!("init");
        tttask::spawn_after(Seconds(1_u32)).unwrap();
        blink_led::spawn_after(Milliseconds(500_u32)).unwrap();
        (init::LateResources { led, pin }, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        defmt::info!("idle");

        // cortex_m::asm::bkpt();

        loop {
            cortex_m::asm::wfi();
        }
    }

    #[task(resources = [led, pin])]
    fn blink_led(cx: blink_led::Context) {
        let led = cx.resources.led;
        let pin = cx.resources.pin;

        if led.is_set_high().unwrap() {
            led.set_low().unwrap();
            pin.set_low().unwrap();
        } else {
            led.set_high().unwrap();
            pin.set_high().unwrap();
        }
        blink_led::spawn_after(Milliseconds(500_u32)).unwrap();
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
