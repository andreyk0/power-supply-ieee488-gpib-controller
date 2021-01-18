use stm32f4xx_hal::time::Hertz;

pub const SYS_FREQ: Hertz = Hertz(96_000_000);
pub const SYS_CYCLES_PER_MILLISECOND: u32 = SYS_FREQ.0 / 1000;
