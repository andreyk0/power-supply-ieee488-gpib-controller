#![deny(unsafe_code)]
#![cfg_attr(not(doc), no_main)]
#![no_std]

use panic_halt as _;

use cortex_m::asm;
use cortex_m_semihosting::dbg;

use stm32f1xx_hal::{delay, gpio, i2c, pac, prelude::*, serial, spi, time, timer, usb};

use embedded_hal::digital::v2::OutputPin;

use rtic::cyccnt::Duration;

use usb_device::{bus, prelude::*};
use usbd_serial::SerialPort;

use embedded_graphics::{
    fonts::{Font6x8, Text},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Circle,
    style::{PrimitiveStyle, TextStyle},
};

use st7920::ST7920;

use power_supply_ieee488_gpib_controller::{consts::*, types::*, usbserial};

#[rtic::app(device = stm32f1xx_hal::stm32,
            peripherals = true,
            monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        led: LedPin,
        usb_serial: usbserial::UsbSerial<'static, usb::UsbBus<usb::Peripheral>>,
        uart_serial: serial::Serial<
            pac::USART2,
            (
                gpio::gpioa::PA2<gpio::Alternate<gpio::PushPull>>,
                gpio::gpioa::PA3<gpio::Input<gpio::Floating>>,
            ),
        >,
        /*
                display: ST7920<
                    spi::Spi<
                        pac::SPI1,
                        spi::Spi1NoRemap,
                        (
                            gpio::gpioa::PA5<gpio::Alternate<gpio::PushPull>>,
                            spi::NoMiso,
                            gpio::gpioa::PA7<gpio::Alternate<gpio::PushPull>>,
                        ),
                        u8,
                    >,
                    gpio::gpioa::PA6<gpio::Alternate<gpio::PushPull>>,
                    gpio::gpioa::PA4<gpio::Alternate<gpio::PushPull>>,
                >,
        */
    }

    #[init(schedule = [blink])]
    fn init(cx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<usb::UsbBus<usb::Peripheral>>> = None;

        let mut cmcore = cortex_m::peripheral::Peripherals::take().unwrap();
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

        #[cfg(dev)]
        dbg!("clocks").unwrap();

        assert!(clocks.usbclk_valid());

        #[cfg(dev)]
        dbg!("gpio").unwrap();

        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = device.GPIOC.split(&mut rcc.apb2);

        #[cfg(dev)]
        dbg!("blink").unwrap();

        let led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

        // Initialize (enable) the monotonic timer (CYCCNT)
        core.DCB.enable_trace();
        // required on Cortex-M7 devices that software lock the DWT (e.g. STM32F7)
        cortex_m::peripheral::DWT::unlock();
        core.DWT.enable_cycle_counter();

        cx.schedule
            .blink(cx.start + Duration::from_cycles(SYS_FREQ.0 / 2))
            .unwrap();

        #[cfg(dev)]
        dbg!("usbp").unwrap();

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

        #[cfg(dev)]
        dbg!("usb_bus").unwrap();

        *USB_BUS = Some(usb::UsbBus::new(usbp));
        let usb_bus = USB_BUS.as_ref().unwrap();

        #[cfg(dev)]
        dbg!("usb_serial").unwrap();

        let usb_serial = usbserial::UsbSerial::new(usb_bus);

        #[cfg(dev)]
        dbg!("uart_serial").unwrap();

        let pin_tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
        let pin_rx = gpioa.pa3;

        let mut uart_serial = serial::Serial::usart2(
            device.USART2,
            (pin_tx, pin_rx),
            &mut afio.mapr,
            serial::Config::default().baudrate(115_200.bps()),
            clocks,
            &mut rcc.apb1,
        );

        uart_serial.listen(serial::Event::Rxne);

        #[cfg(dev)]
        dbg!("display").unwrap();

        let lcd_sck = gpioa.pa5.into_alternate_push_pull(&mut gpioa.crl);
        let lcd_mosi = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
        let lcd_reset = gpioa.pa6.into_push_pull_output(&mut gpioa.crl);
        let lcd_cs = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);

        let lcd_spi = spi::Spi::spi1(
            device.SPI1,
            (lcd_sck, spi::NoMiso, lcd_mosi),
            &mut afio.mapr,
            spi::Mode {
                polarity: spi::Polarity::IdleLow,
                phase: spi::Phase::CaptureOnFirstTransition,
            },
            time::Hertz(600_000),
            clocks,
            &mut rcc.apb2,
        );

        let mut delay = delay::Delay::new(cmcore.SYST, clocks);

        let mut display = ST7920::new(lcd_spi, lcd_reset, Some(lcd_cs), false);

        display.init(&mut delay).expect("could not init display");
        display.clear(&mut delay).expect("could not clear display");

        let c = Circle::new(Point::new(20, 20), 8)
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On));
        let t = Text::new("Hello Rust!", Point::new(40, 16))
            .into_styled(TextStyle::new(Font6x8, BinaryColor::On));

        c.draw(&mut display).unwrap();
        t.draw(&mut display).unwrap();

        display.flush(&mut delay).expect("could not flush display");

        #[cfg(dev)]
        dbg!("init::LateResources").unwrap();
        init::LateResources {
            led,
            usb_serial,
            uart_serial,
        }
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

    #[task(binds = USB_HP_CAN_TX,
        resources = [usb_serial],
        priority = 2)]
    fn usb_tx(cx: usb_tx::Context) {
        cx.resources.usb_serial.poll();
    }

    #[task(binds = USB_LP_CAN_RX0,
        resources = [usb_serial],
        priority = 2)]
    fn usb_rx(cx: usb_rx::Context) {
        cx.resources.usb_serial.poll();
    }

    #[task(binds = USART2,
        resources = [uart_serial],
        priority = 2)]
    fn uart_poll(cx: uart_poll::Context) {}

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
