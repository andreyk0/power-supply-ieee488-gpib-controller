#![deny(unsafe_code)]
#![cfg_attr(not(doc), no_main)]
#![no_std]

use panic_halt as _;

use cortex_m::asm;
//use cortex_m_semihosting::hprintln;

use stm32f1xx_hal::prelude::*;

use rtic::cyccnt::Duration;

use power_supply_ieee488_gpib_controller::{consts::*, types::*};

#[rtic::app(device = stm32f1xx_hal::stm32,
            peripherals = true,
            monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        led: LedPin,
    }

    #[init(schedule = [blink])]
    fn init(cx: init::Context) -> init::LateResources {
        let mut core: rtic::Peripherals = cx.core;
        let device = cx.device;
        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        let _clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(SYS_FREQ)
            .pclk1(36.mhz())
            .freeze(&mut flash.acr);

        //assert!(clocks.usbclk_valid());

        //hprintln!("clocks").unwrap();

        //let mut gpioa = device.GPIOA.split(&mut rcc.apb2);
        //let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = device.GPIOC.split(&mut rcc.apb2);

        //hprintln!("gpio").unwrap();

        let led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

        // Initialize (enable) the monotonic timer (CYCCNT)
        core.DCB.enable_trace();
        // required on Cortex-M7 devices that software lock the DWT (e.g. STM32F7)
        cortex_m::peripheral::DWT::unlock();
        core.DWT.enable_cycle_counter();

        cx.schedule
            .blink(cx.start + Duration::from_cycles(SYS_FREQ.0 / 2))
            .unwrap();

        //hprintln!("init::LateResources").unwrap();
        init::LateResources { led }
    }

    #[idle()]
    fn idle(_ctx: idle::Context) -> ! {
        loop {
            asm::delay(SYS_FREQ.0 / 10000);
        }
    }

    #[task(resources = [led],
           schedule = [blink],
           priority = 1)]
    fn blink(cx: blink::Context) {
        cx.resources.led.toggle().unwrap();
        cx.schedule
            .blink(cx.scheduled + Duration::from_cycles(SYS_FREQ.0 / 2))
            .unwrap();
    }

    // RTIC requires that unused interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks.
    // Full list in  stm32f1::stm32f103::Interrupt
    extern "C" {
        fn EXTI4();
        fn FSMC();
        fn TAMPER();
    }
};
