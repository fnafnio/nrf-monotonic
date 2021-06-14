/// Monotonic Timer based on NRF Timer Instance
///
/// The frequency is fixed at 1MHz
use crate::hal;
use core::sync::atomic::{compiler_fence, Ordering};
use hal::timer::Instance;
use rtic_monotonic::{
    embedded_time::{clock::Error, fraction::Fraction},
    Clock, Instant, Monotonic,
};

pub struct NrfMonotonic<INSTANCE: Instance> {
    timer: INSTANCE,
    ovf: u64,
}

impl<INSTANCE: Instance> NrfMonotonic<INSTANCE> {
    /// Enable the Timer Instance and provide a new `Monotonic` based on this timer
    /// This Monotonic timer is fixed at 1MHz
    const CC_COMPARE: usize = 0;
    const CC_NOW: usize = 1;
    const CC_OVERFLOW: usize = 2;
    pub fn new(instance: INSTANCE) -> Self {
        {
            // set up the peripheral
            let t0 = instance.as_timer0();
            t0.mode.write(|w| w.mode().timer());
            t0.bitmode.write(|w| w.bitmode()._32bit());
            // 1MHz
            t0.prescaler.write(|w| unsafe { w.prescaler().bits(4) });
            t0.tasks_clear.write(|w| w.tasks_clear().set_bit());
        }
        // info!("nrf-monotonic instance created and configured");
        // We do not start the counter here, it is started in `reset`.
        NrfMonotonic {
            timer: instance,
            ovf: 0,
        }
    }

    fn is_overflow(&self) -> bool {
        self.timer.as_timer0().events_compare[Self::CC_OVERFLOW]
            .read()
            .events_compare()
            .bit()
    }

    fn is_compare_match(&self) -> bool {
        self.timer.as_timer0().events_compare[Self::CC_COMPARE]
            .read()
            .events_compare()
            .bit()
    }

    fn clear_overflow_flag(&self) {
        self.timer.as_timer0().events_compare[Self::CC_OVERFLOW]
            .write(|w| w.events_compare().clear_bit());
    }

    fn clear_compare_match_flag(&self) {
        self.timer.as_timer0().events_compare[Self::CC_COMPARE]
            .write(|w| w.events_compare().clear_bit());
    }
}

impl<INSTANCE: Instance> Clock for NrfMonotonic<INSTANCE> {
    type T = u64;

    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        let cnt = {
            let t0 = self.timer.as_timer0();
            t0.tasks_capture[Self::CC_NOW].write(|w| w.tasks_capture().set_bit());
            compiler_fence(Ordering::SeqCst); // is this even needed
            t0.cc[Self::CC_NOW].read().bits()
        };
        trace!("Clock::try_now={}", cnt);
        Ok(Instant::new(cnt as u64 | self.ovf))
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

            // clear events
            t0.events_compare[0].write(|w| w.events_compare().clear_bit());
            t0.events_compare[1].write(|w| w.events_compare().clear_bit());
            t0.events_compare[2].write(|w| w.events_compare().clear_bit());
            t0.events_compare[3].write(|w| w.events_compare().clear_bit());

            // prepare compare registers
            t0.cc[0].reset();
            t0.cc[1].reset();
            t0.cc[2].write(|w| unsafe { w.bits(u32::MAX) }); // so we have an explicit overflow
            t0.cc[3].reset();

            // disable unneeded interrupts
            t0.intenclr
                .write(|w| w.compare1().set_bit().compare3().set_bit());

            // enable copmare match and overflow interrupts
            t0.intenset
                .write(|w| w.compare0().set_bit().compare2().set_bit());

            // start the timer
            t0.tasks_start.write(|w| w.tasks_start().set_bit());
        }
        // info!("nrf-monotonic reset and started");
    }

    fn set_compare(&mut self, val: &Instant<Self>) {
        let dur = *val.duration_since_epoch().integer();

        trace!("Set Compare to {}", dur);
        self.timer.as_timer0().cc[0]
            .write(|w| unsafe { w.cc().bits((dur & u32::MAX as u64) as u32) });
    }

    fn clear_compare_flag(&mut self) {
        if self.is_compare_match() {
            self.clear_compare_match_flag();
            // self.timer.as_timer0().cc[0].reset();
            debug!("Compare flag cleared");
        }
    }

    fn on_interrupt(&mut self) {
        // self.clear_compare_flag();
        if self.is_overflow() {
            self.clear_overflow_flag();
            self.ovf += 0x1_0000_0000u64;
            debug!("Overflow, flag: {:x}", self.ovf);
        }
    }
}
