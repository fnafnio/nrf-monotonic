//! # `Monotonic` implementation based on DWT and SysTick

#![no_std]

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

use hal::timer::Instance;

use rtic_monotonic::{
    embedded_time::{clock::Error, fraction::Fraction},
    Clock, Instant, Monotonic,
};
/// Monotonic Timer based on NRF Timer Instance
///
/// The frequency is fixed at 1MHz
pub struct NrfMonotonic<INSTANCE: Instance> {
    timer: INSTANCE,
    ovf: u64,
}

impl<INSTANCE: Instance> NrfMonotonic<INSTANCE> {
    /// Enable the Timer Instance and provide a new `Monotonic` based on this timer
    /// This Monotonic timer is fixed at 1MHz
    pub fn new(instance: INSTANCE) -> Self {
        {
            // set up the peripheral
            let t0 = instance.as_timer0();
            t0.mode.write(|w| w.mode().timer());
            t0.bitmode.write(|w| w.bitmode()._32bit());
            // 1MHz
            t0.prescaler.write(|w| unsafe { w.prescaler().bits(4) });
            // clear timer on overflow match
            t0.cc[0].reset();
            t0.cc[1].reset();
            t0.cc[2].write(|w| unsafe { w.bits(u32::MAX) });
            t0.cc[3].reset();
            t0.tasks_clear.write(|w| w.tasks_clear().set_bit());
        }

        // We do not start the counter here, it is started in `reset`.
        NrfMonotonic {
            timer: instance,
            ovf: 0,
        }
    }

    fn is_overflow(&self) -> bool {
        self.timer.as_timer0().events_compare[2]
            .read()
            .events_compare()
            .bit()
    }

    fn is_compare_match(&self) -> bool {
        self.timer.as_timer0().events_compare[0]
            .read()
            .events_compare()
            .bit()
    }

    fn clear_overflow_flag(&self) {
        self.timer.as_timer0().events_compare[0].write(|w| w);
    }

    fn clear_compare_match_flag(&self) {
        self.timer.as_timer0().events_compare[2].write(|w| w);
    }
}

impl<INSTANCE: Instance> Clock for NrfMonotonic<INSTANCE> {
    type T = u64;

    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        let cnt = self.timer.read_counter();

        let ovf = if self.is_overflow() {
            0x1_0000_0000u64
        } else {
            0
        };

        Ok(Instant::new(cnt as u64 | ovf as u64))
    }
}

impl<INSTANCE: Instance> Monotonic for NrfMonotonic<INSTANCE> {
    const DISABLE_INTERRUPT_ON_EMPTY_QUEUE: bool = false;

    unsafe fn reset(&mut self) {
        self.ovf = 0;
        {
            let t0 = self.timer.as_timer0();
            t0.tasks_stop.write(|w| w.tasks_stop().set_bit());
            t0.tasks_clear.write(|w| w.tasks_clear().set_bit());
            t0.events_compare[0].reset();
            t0.events_compare[1].reset();
            t0.events_compare[2].reset();
            t0.events_compare[3].reset();
            t0.intenset.write(|w| w.compare0().set().compare2().set());
            t0.tasks_start.write(|w| w.tasks_start().set_bit());
        }
    }

    fn set_compare(&mut self, val: &Instant<Self>) {
        // The input `val` is in the timer, but the SysTick is a down-counter.
        // We need to convert into its domain.
        let now: Instant<Self> = self.try_now().unwrap(); // infallible
        let max = u64::MAX;

        let dur = match val.checked_duration_since(&now) {
            None => 1, // In the past

            // ARM Architecture Reference Manual says:
            // "Setting SYST_RVR to zero has the effect of
            // disabling the SysTick counter independently
            // of the counter enable bit.", so the min is 1
            Some(x) => max.min(*x.integer()).max(1u64),
        };

        self.timer.as_timer0().cc[0]
            .write(|w| unsafe { w.cc().bits((dur & u32::MAX as u64) as u32) });
    }

    fn clear_compare_flag(&mut self) {
        if self.is_compare_match() {
            self.clear_compare_match_flag();
        }
    }

    fn on_interrupt(&mut self) {
        // maybe this check can be left out?
        if self.is_overflow() {
            self.clear_overflow_flag();
            self.ovf += 1 << 32;
        }
    }
}
