#![cfg_attr(not(doc), no_main)]
#![no_std]

use panic_halt as _;

use cortex_m::asm;
use cortex_m_semihosting::*;

use stm32f1xx_hal::{prelude::*, serial, spi, time, usb};

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi as espi;

use rtic::cyccnt::Duration;

use usb_device::bus;

use st7920::ST7920;

use heapless::{consts::*, Vec};

use power_supply_ieee488_gpib_controller::*;
use power_supply_ieee488_gpib_controller::{
    display::*, model::*, prelude::*, sdcard::*, time::*, uart_serial::*,
};

#[rtic::app(device = stm32f1xx_hal::stm32,
            peripherals = true,
            monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        ps: PS,
        led: LedPin,
        usb_serial: UsbSerial,
        uart_serial: UartSerial,
        display: Display,
        sdcard: SDCard,
    }

    #[init(schedule = [blink])]
    fn init(cx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<usb::UsbBus<usb::Peripheral>>> = None;

        let mut core: rtic::Peripherals = cx.core;
        let device = cx.device;
        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();
        let mut afio = device.AFIO.constrain(&mut rcc.apb2);

        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(SYS_FREQ)
            .pclk1(36.mhz())
            .freeze(&mut flash.acr);

        ifcfg!("bin_info", hprintln!("clocks"));

        assert!(clocks.usbclk_valid());

        ifcfg!("bin_info", hprintln!("gpio"));

        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = device.GPIOC.split(&mut rcc.apb2);

        let mut ps = PS::new();

        ifcfg!("bin_info", hprintln!("display"));

        let lcd_sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
        let lcd_mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
        let lcd_reset = gpioa.pa6.into_push_pull_output(&mut gpioa.crl);
        let lcd_cs = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);

        let lcd_spi = spi::Spi::spi1(
            device.SPI1,
            (lcd_sck, spi::NoMiso, lcd_mosi),
            &mut afio.mapr,
            espi::MODE_0,
            time::Hertz(600_000),
            clocks,
            &mut rcc.apb2,
        );

        let mut display =
            Display::new(ST7920::new(lcd_spi, lcd_reset, Some(lcd_cs), false)).unwrap();

        ps.act(Act::UILoading("blink"));
        display.render(&ps).unwrap();

        let led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

        // Initialize (enable) the monotonic timer (CYCCNT)
        core.DCB.enable_trace();
        // required on Cortex-M7 devices that software lock the DWT (e.g. STM32F7)
        cortex_m::peripheral::DWT::unlock();
        core.DWT.enable_cycle_counter();

        cx.schedule
            .blink(cx.start + Duration::from_cycles(SYS_FREQ.0 / 2))
            .unwrap();

        ps.act(Act::UILoading("usbp"));
        display.render(&ps).unwrap();

        // BluePill board has a pull-up resistor on the D+ line.
        // Pull the D+ pin down to send a RESET condition to the USB bus.
        // This forced reset is needed only for development, without it host
        // will not reset your device when you upload new firmware.
        let mut usb_dp = gpioa.pa12.into_push_pull_output(&mut gpioa.crh);
        usb_dp.set_low().unwrap();
        asm::delay(clocks.sysclk().0 / 10);

        let usb_dm = gpioa.pa11;
        let usb_dp = usb_dp.into_floating_input(&mut gpioa.crh);

        let usbp = usb::Peripheral {
            usb: device.USB,
            pin_dm: usb_dm,
            pin_dp: usb_dp,
        };

        ps.act(Act::UILoading("usb_bus"));
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("usb_bus"));

        *USB_BUS = Some(usb::UsbBus::new(usbp));
        let usb_bus = USB_BUS.as_ref().unwrap();

        ps.act(Act::UILoading("usb_serial"));
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("usb_serial"));

        let usb_serial = UsbSerial::new(usb_bus);

        ps.act(Act::UILoading("uart_serial"));
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("uart_serial"));

        let pin_tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
        let pin_rx = gpioa.pa3;

        let mut uart_serial = UartSerial::new(serial::Serial::usart2(
            device.USART2,
            (pin_tx, pin_rx),
            &mut afio.mapr,
            serial::Config::default().baudrate(115_200.bps()),
            clocks,
            &mut rcc.apb1,
        ));

        uart_serial.init();

        ps.act(Act::UILoading("sd_card"));
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("sd_card"));

        let sd_sck = gpiob.pb13.into_alternate_push_pull(&mut gpiob.crh);
        let sd_miso = gpiob.pb14;
        let sd_mosi = gpiob.pb15.into_alternate_push_pull(&mut gpiob.crh);
        let sd_cs = gpiob.pb12.into_push_pull_output(&mut gpiob.crh);

        let sd_spi = spi::Spi::spi2(
            device.SPI2,
            (sd_sck, sd_miso, sd_mosi),
            espi::MODE_0,
            time::Hertz(400_000),
            clocks,
            &mut rcc.apb1,
        );

        let sdcard = SDCard::new(embedded_sdmmc::Controller::new(
            embedded_sdmmc::SdMmcSpi::new(sd_spi, sd_cs),
            DummyTimeSource {},
        ));

        ps.act(Act::UILoading("resources"));
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("resources"));

        init::LateResources {
            ps,
            led,
            usb_serial,
            uart_serial,
            display,
            sdcard,
        }
    }

    #[idle(resources = [ps, display, sdcard, usb_serial, uart_serial])]
    fn idle(mut cx: idle::Context) -> ! {
        cx.resources.usb_serial.lock(|s| s.poll());

        let ps = cx.resources.ps;
        let display = cx.resources.display;

        ps.act(Act::UILoading("BOOT"));
        display.render(ps).unwrap();

        let sdc = cx.resources.sdcard;

        cx.resources
            .uart_serial
            .lock(|us| {
                sdc.send_file("BOOT", |buf| us.write_buf(buf))
                    .and_then(|_| us.flush())
            })
            .map_err(|e| ps.act(Act::ShowError(e)))
            .unwrap();

        loop {
            display.render(ps).unwrap();
            cx.resources.usb_serial.lock(|s| s.poll());
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

    #[task(binds = USB_HP_CAN_TX,
            resources = [usb_serial],
            priority = 2)]
    fn usb_tx(cx: usb_tx::Context) {
        cx.resources.usb_serial.poll();
    }
    #[task(binds = USB_LP_CAN_RX0,
            resources = [usb_serial, uart_serial],
            priority = 2)]
    fn usb_rx(cx: usb_rx::Context) {
        cx.resources.usb_serial.poll();

        let mut buf: [u8; 16] = [0; 16];
        match cx.resources.usb_serial.read(&mut buf) {
            Ok(s) if s > 0 => {
                cx.resources.uart_serial.write_buf(&buf).unwrap();
                cx.resources.uart_serial.flush().unwrap()
            }
            _ => {}
        }
    }

    #[task(binds = USART2,
        resources = [uart_serial, usb_serial],
        priority = 2)]
    fn uart_poll(cx: uart_poll::Context) {
        let mut buf: Vec<u8, U16> = Vec::new();
        cx.resources.uart_serial.fill_buf(&mut buf).unwrap();

        // Ignore USB errors (may not be connected)
        cx.resources.usb_serial.write(&buf).map_or((), |_| ());
    }

    // RTIC requires that unused interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks.
    // Full list in  stm32f1::stm32f103::Interrupt
    extern "C" {
        fn EXTI4();
        fn FSMC();
        fn TAMPER();
        fn CAN_RX1();
        fn CAN_SCE();
    }
};
