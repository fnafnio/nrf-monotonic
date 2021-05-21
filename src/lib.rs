//! # `Monotonic` implementation based on DWT and SysTick

#![no_std]

use hal::timer::Instance;
use nrf52840_hal as hal;
use rtic_monotonic::{
    embedded_time::{clock::Error, fraction::Fraction},
    Clock, Instant, Monotonic,
};
/// DWT and Systick combination implementing `embedded_time::Clock` and `rtic_monotonic::Monotonic`
///
/// The frequency of the DWT and SysTick is encoded using the parameter `FREQ`.
pub struct NrfMonotonic<INSTANCE: Instance> {
    timer: INSTANCE,
}

impl<INSTANCE: Instance> NrfMonotonic<INSTANCE> {
    /// Enable the DWT and provide a new `Monotonic` based on DWT and SysTick.
    ///
    /// Note that the `sysclk` parameter should come from e.g. the HAL's clock generation function
    /// so the real speed and the declared speed can be compared.
    pub fn new(instance: INSTANCE) -> Self {
        {
            // set up the peripheral
            let t0 = instance.as_timer0();
            t0.bitmode.write(|w| w.bitmode()._32bit());
            t0.events_compare[0].reset();
            t0.mode.write(|w| w.mode().timer());
            // 1MHz
            t0.prescaler.write(|w| unsafe { w.prescaler().bits(4) });
            t0.tasks_clear.write(|w| w.tasks_clear().set_bit());
        }
        // We do not start the counter here, it is started in `reset`.

        NrfMonotonic { timer: instance }
    }
}

impl<INSTANCE: Instance> Clock for NrfMonotonic<INSTANCE> {
    type T = u32;

    const SCALING_FACTOR: Fraction = Fraction::new(1, 1);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        Ok(Instant::new(self.timer.read_counter()))
    }
}

impl<INSTANCE: Instance> Monotonic for NrfMonotonic<INSTANCE> {
    const DISABLE_INTERRUPT_ON_EMPTY_QUEUE: bool = false;

    unsafe fn reset(&mut self) {
        {
            let t0 = self.timer.as_timer0();
            t0.tasks_stop.write(|w| w.tasks_stop().set_bit());
            t0.tasks_clear.write(|w| w.tasks_clear().set_bit());
            t0.tasks_start.write(|w| w.tasks_start().set_bit());
        }
    }

    fn set_compare(&mut self, val: &Instant<Self>) {
        // The input `val` is in the timer, but the SysTick is a down-counter.
        // We need to convert into its domain.
        let now: Instant<Self> = Instant::new(self.timer.read_counter());

        let max = u32::MAX;

        let dur = match val.checked_duration_since(&now) {
            None => 1, // In the past

            // ARM Architecture Reference Manual says:
            // "Setting SYST_RVR to zero has the effect of
            // disabling the SysTick counter independently
            // of the counter enable bit.", so the min is 1
            Some(x) => max.min(*x.integer()).max(1),
        };

        self.timer.as_timer0().cc[0].write(|w| unsafe { w.cc().bits(dur) });
    }

    fn clear_compare_flag(&mut self) {
        self.timer.timer_reset_event();
        // NOOP with SysTick interrupt
    }
}
