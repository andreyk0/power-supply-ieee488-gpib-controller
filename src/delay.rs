//! # Asm us delay implementation

use cortex_m::asm::delay;
use embedded_hal::blocking::delay::DelayUs;

use crate::consts::*;

pub struct AsmDelay {}

impl DelayUs<u32> for AsmDelay {
    fn delay_us(&mut self, us: u32) {
        delay((SYS_FREQ.0 / 1_000_000u32) * us)
    }
}
