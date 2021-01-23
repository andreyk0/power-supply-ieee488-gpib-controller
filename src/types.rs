use stm32f1xx_hal::gpio::*;

pub type LedPin = gpioc::PC13<Output<PushPull>>;
