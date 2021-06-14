#[allow(unused)]
#[cfg(feature = "defmt-impl")]
use crate::fmt_helpers::*;
use crate::hal;

use hal::rtc::Instance as RtcInstance;
use rtic_monotonic::embedded_time::clock::Error as ClockError;
use rtic_monotonic::{Clock, Fraction, Instant, Monotonic};

pub struct RtcMono<RTC: RtcInstance> {
    rtc: RTC,
    ovfl: u32,
}

fn calc_now(period: u32, counter: u32) -> u64 {
    ((period as u64) << 23) + ((counter ^ ((period & 1) << 23)) as u64)
}

impl<RTC: RtcInstance> RtcMono<RTC> {
    pub fn new(rtc: RTC) -> Self {
        rtc.tasks_clear.write(|w| w.tasks_clear().set_bit());
        rtc.intenset.write(|w| w.ovrflw().set_bit());
        rtc.prescaler.write(|w| unsafe { w.prescaler().bits(0) });
        Self { rtc, ovfl: 0 }
    }

    #[inline(always)]
    fn is_overflow(&self) -> bool {
        self.rtc.events_ovrflw.read().events_ovrflw().bit()
    }

    #[inline(always)]
    fn clear_overflow_flag(&self) {
        self.rtc
            .events_ovrflw
            .write(|w| w.events_ovrflw().clear_bit());
    }

    // #[inline(always)]
    // fn is_next_period(&self) -> bool {
    //     self.rtc.events_compare[3]
    //         .read()
    //         .events_compare()
    //         .bit_is_set()
    // }

    // #[inline(always)]
    // fn clear_next_period_flag(&self) {
    //     self.rtc.events_compare[3].write(|w| w.events_compare().clear_bit())
    // }

    // fn next_period(&self) {
    // let t = (self.ovfl as u64) << 23;
    // }
}
impl<RTC: RtcInstance> Clock for RtcMono<RTC> {
    type T = u64;

    const SCALING_FACTOR: Fraction = Fraction::new(1, 32_768);

    fn try_now(&self) -> Result<Instant<Self>, ClockError> {
        let cnt = self.rtc.counter.read().bits();
        let now = calc_now(self.ovfl, cnt);
        trace!("now {:x}", now);
        Ok(Instant::new(now))
    }
}

impl<RTC: RtcInstance> Monotonic for RtcMono<RTC> {
    fn on_interrupt(&mut self) {
        trace!("RTC interrupt");
        if self.is_overflow() {
            debug!("is overflow");
            self.clear_overflow_flag();
            self.ovfl += 1;
        }

        // if self.is_next_period() {
        //     debug!("ts next period");
        //     self.clear_next_period_flag();
        // }
    }

    fn enable_timer(&mut self) {}

    fn disable_timer(&mut self) {}

    const DISABLE_INTERRUPT_ON_EMPTY_QUEUE: bool = false;

    unsafe fn reset(&mut self) {
        {
            // self.rtc.evtenset.write(|w| w.ovrflw().set_bit());
            self.rtc.cc[3].write(|w| unsafe { w.bits(0x0080_0000) });

            self.rtc.tasks_clear.write(|w| w.tasks_clear().set_bit());
            self.rtc.tasks_start.write(|w| w.tasks_start().set_bit());

            // wait for counter to clear
            while self.rtc.counter.read().bits() != 0 {}

            self.rtc.evtenset.write(|w| {
                w.compare0()
                    .set_bit()
                    .compare3()
                    .set_bit()
                    .ovrflw()
                    .set_bit()
            });

            self.rtc.intenset.write(|w| {
                w.ovrflw().set_bit().compare0().set_bit()
                // .compare3()
                // .set_bit()
            });
        }
    }

    fn set_compare(&mut self, instant: &Instant<Self>) {
        // let now = self.try_now().unwrap();
        // let dur = instant.checked_duration_since(&now);
        let dur = instant.duration_since_epoch();
        let ticks = *dur.integer();
        trace!("ticks: {}", ticks);
        self.rtc.cc[0].write(|w| unsafe { w.bits(((ticks + 3) as u32) & 0x00FF_FFFF) })
    }

    fn clear_compare_flag(&mut self) {
        // trace!("Compare flag cleared");
        self.rtc.events_compare[0].write(|w| w.events_compare().clear_bit());
    }
}
