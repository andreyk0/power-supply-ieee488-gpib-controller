//! # Asm us delay implementation

use cortex_m::asm::delay;
use embedded_hal::blocking::delay::DelayUs;

use crate::consts::*;

pub struct AsmDelay {}

const CYCLES: u32 = SYS_FREQ.0 / 1_000_000u32;

impl DelayUs<u32> for AsmDelay {
    #[inline]
    fn delay_us(&mut self, us: u32) {
        delay(CYCLES * us)
    }
}
