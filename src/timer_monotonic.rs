/// Monotonic Timer based on NRF Timer Instance
///
/// The frequency is fixed at 1MHz
use crate::hal;
use hal::{pac::timer0::RegisterBlock as TimerRegister, timer::Instance};
use rtic_monotonic::Monotonic;
pub const TIMER_HZ: u32 = 1_000_000;

pub struct NrfMonotonic<INSTANCE: Instance> {
    timer: INSTANCE,
    ovf: u64,
}

impl<INSTANCE: Instance> NrfMonotonic<INSTANCE> {
    const OVFLOW_REGISTER: u32 = u32::MAX >> 1;
    const OVFLOW_INCREMENT: u64 = (Self::OVFLOW_REGISTER as u64) + 1;
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
            t0.prescaler.write(|w| unsafe { w.prescaler().bits(4) });

            start_timer0(t0);
        }
        // info!("nrf-monotonic instance created and configured");
        // We do not start the counter here, it is started in `reset`.
        NrfMonotonic {
            timer: instance,
            ovf: 0,
        }
    }

    #[inline(always)]
    fn is_overflow(&self) -> bool {
        self.timer.as_timer0().events_compare[Self::CC_OVERFLOW]
            .read()
            .events_compare()
            .bit()
    }

    #[inline(always)]
    fn is_compare_match(&self) -> bool {
        self.timer.as_timer0().events_compare[Self::CC_COMPARE]
            .read()
            .events_compare()
            .bit()
    }

    #[inline(always)]
    fn clear_overflow_flag(&self) {
        self.timer.as_timer0().events_compare[Self::CC_OVERFLOW]
            .write(|w| w.events_compare().clear_bit());
    }

    #[inline(always)]
    fn clear_compare_match_flag(&self) {
        self.timer.as_timer0().events_compare[Self::CC_COMPARE]
            .write(|w| w.events_compare().clear_bit());
    }
}

#[inline(always)]
fn start_timer0(t0: &TimerRegister) {
    t0.tasks_stop.write(|w| w.tasks_stop().set_bit());
    t0.tasks_clear.write(|w| w.tasks_clear().set_bit());
    enable_interrupts(t0);
    t0.tasks_start.write(|w| w.tasks_start().set_bit());
}

#[inline(always)]
fn stop_timer0(t0: &TimerRegister) {
    disable_interrupts(t0);
    t0.tasks_stop.write(|w| w.tasks_stop().set_bit());
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
            t0.cc[2].write(|w| unsafe { w.bits(Self::OVFLOW_REGISTER) }); // so we have an explicit overflow
            t0.cc[3].reset();

            t0.shorts.write(|w| w.compare2_clear().set_bit());

            // disable unneeded interrupts
            disable_interrupts(t0);

            // enable copmare match and overflow interrupts
            enable_interrupts(t0);

            // start the timer
            t0.tasks_start.write(|w| w.tasks_start().set_bit());
        }
    }

    fn set_compare(&mut self, val: Self::Instant) {
        let now = self.now();
        let max = u32::MAX;

        let dur = match val.checked_duration_since(now) {
            None => 1, // in the past
            Some(x) => max.min(x.ticks()).max(1),
        };

        self.timer.as_timer0().cc[0]
            .write(|w| unsafe { w.cc().bits((dur & (Self::OVFLOW_REGISTER)) as u32) });
    }

    fn clear_compare_flag(&mut self) {
        if self.is_compare_match() {
            self.clear_compare_match_flag();
            trace!("Compare flag cleared");
        }
    }

    fn on_interrupt(&mut self) {
        // self.clear_compare_flag();
        if self.is_overflow() {
            self.clear_overflow_flag();
            self.ovf += Self::OVFLOW_INCREMENT;
            debug!("Overflow, flag: {:x}", self.ovf);
        }
    }

    fn enable_timer(&mut self) {
        {
            let t0 = self.timer.as_timer0();
            start_timer0(t0);
            enable_interrupts(t0);
        }
    }

    fn disable_timer(&mut self) {
        {
            let t0 = self.timer.as_timer0();
            disable_interrupts(t0);
            stop_timer0(t0);
        }
    }

    type Instant = fugit::TimerInstantU32<{ TIMER_HZ }>;
    type Duration = fugit::TimerDurationU32<{ TIMER_HZ }>;

    fn now(&mut self) -> Self::Instant {
        let t0 = self.timer.as_timer0();
        t0.tasks_capture[Self::CC_NOW].write(|w| w.tasks_capture().set_bit());
        Self::Instant::from_ticks(t0.cc[Self::CC_NOW].read().bits())
    }

    fn zero() -> Self::Instant {
        Self::Instant::from_ticks(0)
    }
}

#[inline(always)]
fn disable_interrupts(t0: &TimerRegister) {
    t0.intenclr
        .write(|w| w.compare1().set_bit().compare3().set_bit());
}

#[inline(always)]
fn enable_interrupts(t0: &TimerRegister) {
    t0.intenset
        .write(|w| w.compare0().set_bit().compare2().set_bit());
}
