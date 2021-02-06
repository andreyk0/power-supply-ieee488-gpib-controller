#![cfg_attr(not(doc), no_main)]
#![no_std]

use panic_halt as _;

use cortex_m::asm;
use cortex_m_semihosting::*;

use stm32f1xx_hal::{delay, gpio, pac, prelude::*, serial, spi, time, usb};

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi as espi;

use rtic::cyccnt::Duration;

use usb_device::bus;

use embedded_graphics::{
    fonts::{Font6x8, Text},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Circle,
    style::{PrimitiveStyle, TextStyle},
};

use st7920::ST7920;

use heapless::consts::*;
use heapless::Vec;

use power_supply_ieee488_gpib_controller::{
    consts::*, delay::AsmDelay, sdcard, types::*, usbserial,
};

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

        hprintln!("clocks").unwrap();

        assert!(clocks.usbclk_valid());

        hprintln!("gpio").unwrap();

        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);
        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);
        let mut gpioc = device.GPIOC.split(&mut rcc.apb2);

        hprintln!("blink").unwrap();

        let led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

        // Initialize (enable) the monotonic timer (CYCCNT)
        core.DCB.enable_trace();
        // required on Cortex-M7 devices that software lock the DWT (e.g. STM32F7)
        cortex_m::peripheral::DWT::unlock();
        core.DWT.enable_cycle_counter();

        cx.schedule
            .blink(cx.start + Duration::from_cycles(SYS_FREQ.0 / 2))
            .unwrap();

        hprintln!("usbp").unwrap();

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

        hprintln!("usb_bus").unwrap();

        *USB_BUS = Some(usb::UsbBus::new(usbp));
        let usb_bus = USB_BUS.as_ref().unwrap();

        hprintln!("usb_serial").unwrap();

        let usb_serial = usbserial::UsbSerial::new(usb_bus);

        hprintln!("uart_serial").unwrap();

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

        hprintln!("display").unwrap();

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

        let mut delay = AsmDelay {};

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

        hprintln!("sdcard").unwrap();

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

        let mut sd_cont = embedded_sdmmc::Controller::new(
            embedded_sdmmc::SdMmcSpi::new(sd_spi, sd_cs),
            sdcard::DummyTimeSource {},
        );

        hprintln!("Init SD card...").unwrap();

        let mut sdres = sd_cont.device().init();
        while sdres.is_err() {
            hprintln!("{:?}!", sdres).unwrap();
            sdres = sd_cont.device().init();
        }

        match sdres {
            Ok(_) => {
                hprintln!("SD init OK!").unwrap();
                match sd_cont.device().card_size_bytes() {
                    Ok(size) => hprintln!("Card size {}", size).unwrap(),
                    Err(e) => hprintln!("Err: {:?}", e).unwrap(),
                }
                match sd_cont.get_volume(embedded_sdmmc::VolumeIdx(0)) {
                    Ok(mut v) => {
                        hprintln!("Volume 0 {:?}", v).unwrap();
                        let r = sd_cont.open_root_dir(&v).unwrap();
                        let mut bootf = sd_cont
                            .open_file_in_dir(
                                &mut v,
                                &r,
                                "BOOT",
                                embedded_sdmmc::filesystem::Mode::ReadOnly,
                            )
                            .unwrap();

                        let mut buf: [u8; 64] = [0; 64];
                        sd_cont.read(&v, &mut bootf, &mut buf).unwrap();

                        hprintln!("boot buf: {:?}", buf).unwrap();

                        for c in &buf[0..64] {
                            uart_serial.write(*c).map_or((), |_| ())
                        }
                        uart_serial.flush().map_or((), |_| ());
                    }
                    Err(e) => hprintln!("Err: {:?}", e).unwrap(),
                }
            }
            Err(e) => hprintln!("{:?}!", e).unwrap(),
        }

        hprintln!("init::LateResources").unwrap();
        init::LateResources {
            led,
            usb_serial,
            uart_serial,
        }
    }

    #[idle(resources = [usb_serial])]
    fn idle(mut cx: idle::Context) -> ! {
        let foo = "foo\n";

        loop {
            asm::delay(3 * SYS_FREQ.0);

            //            let mut data: [u8; 8] = [0; 8];

            /*
            cx.resources.usb_serial.lock(|s| match s.read(&mut data) {
                Ok(_) => {
                    //s.write("foo".as_bytes()).unwrap();
                }
                Err(_) => {}
            });
             */

            //cx.resources.usb_serial.lock(|s| s.write(foo.as_bytes()));

            //hprintln!("R: {:?}", data);
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
                for c in &buf[0..s] {
                    cx.resources.uart_serial.write(*c).map_or((), |_| ())
                }
                cx.resources.uart_serial.flush().map_or((), |_| ());
            }
            _ => {}
        }
    }

    #[task(binds = USART2,
        resources = [uart_serial, usb_serial],
        priority = 2)]
    fn uart_poll(cx: uart_poll::Context) {
        let mut buf: Vec<u8, U16> = Vec::new();

        while cx
            .resources
            .uart_serial
            .read()
            .map_or(false, |c| buf.push(c).map_or(false, |_| true))
        { /**/ }

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
