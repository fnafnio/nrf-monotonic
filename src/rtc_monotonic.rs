use core::convert::TryInto;
use core::sync::atomic::compiler_fence;

use crate::hal;

use hal::rtc::Instance as RtcInstance;
use hal::timer::Instance as TimerInstance;
use rtic_monotonic::{Clock, Fraction, Instant, Microseconds, Monotonic};

pub struct RtcMonotonic<RTC: RtcInstance, TIM: TimerInstance> {
    tim: TIM,
    rtc: RTC,
    ovf: u64,
}

impl<RTC: RtcInstance, TIM: TimerInstance> RtcMonotonic<RTC, TIM> {
    pub fn new(rtc: RTC, tim: TIM) -> Self {
        {
            // TIM setup
            let t0 = tim.as_timer0();
            t0.mode.write(|w| w.mode().timer());
            t0.bitmode.write(|w| w.bitmode()._32bit());
            // 1MHz
            t0.prescaler.write(|w| unsafe { w.prescaler().bits(4) });
            t0.tasks_clear.write(|w| w.tasks_clear().set_bit());
        }
        {
            // RTC setup
            rtc.tasks_clear.write(|w| w.tasks_clear().set_bit());
            rtc.intenset.write(|w| w.ovrflw().set_bit());
            rtc.prescaler.write(|w| unsafe { w.prescaler().bits(0) });
            // rtc.
        }
        Self { tim, rtc, ovf: 0 }
    }

    fn is_overflow(&self) -> bool {
        self.rtc.events_ovrflw.read().events_ovrflw().bit()
    }

    fn clear_overflow_flag(&self) {
        self.rtc
            .events_ovrflw
            .write(|w| w.events_ovrflw().clear_bit());
    }
}

impl<RTC: RtcInstance, TIM: TimerInstance> Clock for RtcMonotonic<RTC, TIM> {
    type T = u64;

    const SCALING_FACTOR: Fraction = Fraction::new(1, 32_768);

    fn try_now(
        &self,
    ) -> Result<rtic_monotonic::Instant<Self>, rtic_monotonic::embedded_time::clock::Error> {
        let cnt = self.rtc.counter.read().bits();
        let cnt = cnt as u64 | self.ovf;
        trace!("Now: {:010x}", cnt);
        Ok(Instant::new(cnt))
    }
}

impl<RTC: RtcInstance, TIM: TimerInstance> Monotonic for RtcMonotonic<RTC, TIM> {
    const DISABLE_INTERRUPT_ON_EMPTY_QUEUE: bool = false;

    unsafe fn reset(&mut self) {
        self.ovf = 0;
        {
            let t0 = self.tim.as_timer0();
            t0.tasks_stop.write(|w| w.tasks_stop().set_bit());
            t0.tasks_clear.write(|w| w.tasks_clear().set_bit());

            // clear events
            t0.events_compare[0].write(|w| w.events_compare().clear_bit());
            t0.events_compare[1].write(|w| w.events_compare().clear_bit());
            t0.events_compare[2].write(|w| w.events_compare().clear_bit());
            t0.events_compare[3].write(|w| w.events_compare().clear_bit());

            // disable unneeded interrupts
            t0.intenclr.write(|w| {
                w.compare1()
                    .set_bit()
                    .compare2()
                    .set_bit()
                    .compare3()
                    .set_bit()
            });

            // enable copmare match and overflow interrupts
            t0.intenset.write(|w| w.compare0().set_bit());

            // don't start the timer, it is only needed for scheduling/ after setting compare
            t0.tasks_start.write(|w| w.tasks_start().set_bit());
        }
        {
            self.rtc.evtenset.write(|w| w.ovrflw().set_bit());
            self.rtc.tasks_start.write(|w| w.tasks_start().set_bit());
        }
    }

    fn set_compare(&mut self, instant: &rtic_monotonic::Instant<Self>) {
        let now = self.try_now().unwrap();

        let dur = instant.checked_duration_since(&now);

        let micros: Microseconds = match dur {
            Some(x) => match x.try_into() {
                Ok(Microseconds(0u32)) | Err(_) => {
                    trace!("failed or 0");
                    Microseconds(1)
                }
                Ok(x) => x,
            },
            None => Microseconds(1),
        };

        // need to check for an overflowed value. Horribly inefficient as well
        let micros = if micros.0 > 0x00FF_FFFF {
            trace!("New micros value {}", !micros.0);
            Microseconds((!micros.0) & 0x00FF_FFFF)
        } else {
            micros
        };

        compiler_fence(core::sync::atomic::Ordering::SeqCst);
        trace!("Set compare reg to {}", micros.0);
        let ticks = micros.0;
        {
            let t0 = self.tim.as_timer0();
            t0.cc[0].write(|w| unsafe { w.bits(ticks as u32 & 0x00FF_FFFF) });
            compiler_fence(core::sync::atomic::Ordering::SeqCst);
            t0.tasks_clear.write(|w| w.tasks_clear().set_bit());
        }
    }

    fn clear_compare_flag(&mut self) {
        {
            trace!("Clear compare flag");
            let t0 = self.tim.as_timer0();
            t0.events_compare[0].write(|w| w.events_compare().clear_bit());
        }
    }

    fn on_interrupt(&mut self) {
        trace!("RTC interrupt");

        if self.is_overflow() {
            debug!("RTC Overflow");
            self.clear_overflow_flag();
            self.ovf += 0x0100_0000;
        }
    }

    fn enable_timer(&mut self) {}

    fn disable_timer(&mut self) {}
}
