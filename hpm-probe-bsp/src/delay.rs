use crate::clock::Clocks;
use core::sync::atomic::{AtomicU32, Ordering};
use hpm_ral::mchtmr;
use hpm_ral::{modify_reg, read_reg, write_reg};

pub struct Delay {
    mchtmr: mchtmr::MCHTMR,
    base_clock: AtomicU32,
}

impl Delay {
    pub fn new(mchtmr: mchtmr::MCHTMR) -> Self {
        // Set reload values
        write_reg!(mchtmr, mchtmr, MTIMECMP, 0xffffffff);

        Delay {
            mchtmr,
            base_clock: AtomicU32::new(0),
        }
    }

    pub fn set_base_clock(&self, clocks: &Clocks) {
        self.base_clock
            .store(clocks.get_clk_mchtmr0_freq(), Ordering::SeqCst);
    }

    pub fn delay_us(&self, us: u32) {
        assert!(us < 1_000_000);

        let base_clock = self.base_clock.load(Ordering::SeqCst);
        assert!(base_clock > 0);

        let ticks = (us as u64) * (base_clock as u64) / 1_000_000;
        self.delay_ticks(ticks as u32);
    }

    pub fn calc_period_ticks(&self, frequency: u32) -> u32 {
        let base_clock = self.base_clock.load(Ordering::SeqCst);
        assert!(base_clock > 0);

        base_clock / frequency
    }

    pub fn delay_ticks(&self, mut ticks: u32) {
        let mut last = self.get_current();
        loop {
            let now = self.get_current();
            let delta = now.wrapping_sub(last) & 0xffffffff;

            if delta >= ticks {
                break;
            } else {
                ticks -= delta;
                last = now;
            }
        }
    }

    pub fn delay_ticks_from_last(&self, mut ticks: u32, mut last: u32) -> u32 {
        loop {
            let now = self.get_current();
            let delta = last.wrapping_sub(now) & 0xffffffff;

            if delta >= ticks {
                break now;
            } else {
                ticks -= delta;
                last = now;
            }
        }
    }

    #[inline(always)]
    pub fn get_current(&self) -> u32 {
        read_reg!(mchtmr, self.mchtmr, MTIME) as u32
    }
}
