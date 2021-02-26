#![cfg_attr(not(doc), no_main)]
#![no_std]

use panic_halt as _;

use core::fmt::Write;

use cortex_m::asm;
use cortex_m_semihosting::*;

use stm32f1xx_hal::{prelude::*, serial, spi, time, usb};

use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi as espi;

use rtic::cyccnt::Duration;
use rtic::Mutex;

use usb_device::bus;

use st7920::ST7920;

use heapless::{consts::*, String, Vec};

use power_supply_ieee488_gpib_controller::*;
use power_supply_ieee488_gpib_controller::{
    button::*, display::*, line::*, model::*, prelude::*, protocol::*, rotary_encoder::*,
    sdcard::*, time::*, uart_serial::*,
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

        uart_rx_buf: Vec<u8, U32>,
        usb_rx_buf: Vec<u8, U32>,

        query: Option<Query>,
        query_idx: usize,

        btn_pause: Button<PauseButtonPin>,
        btn_encoder: Button<EncoderButtonPin>,

        rotary_encoder: RotaryEncoder,
    }

    #[init(schedule = [ping])]
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

        ps.set_ui_loading("ping");
        display.render(&ps).unwrap();

        let led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

        // Initialize (enable) the monotonic timer (CYCCNT)
        core.DCB.enable_trace();
        // required on Cortex-M7 devices that software lock the DWT (e.g. STM32F7)
        cortex_m::peripheral::DWT::unlock();
        core.DWT.enable_cycle_counter();

        cx.schedule
            .ping(cx.start + Duration::from_cycles(SYS_FREQ.0 / 2))
            .unwrap();

        ps.set_ui_loading("usbp");
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

        ps.set_ui_loading("usb_bus");
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("usb_bus"));

        *USB_BUS = Some(usb::UsbBus::new(usbp));
        let usb_bus = USB_BUS.as_ref().unwrap();

        ps.set_ui_loading("usb_serial");
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("usb_serial"));

        let usb_serial = UsbSerial::new(usb_bus);

        ps.set_ui_loading("uart_serial");
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("uart_serial"));

        let pin_tx = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);
        let pin_rx = gpioa.pa3;

        let mut uart_serial = UartSerial::new(serial::Serial::usart2(
            device.USART2,
            (pin_tx, pin_rx),
            &mut afio.mapr,
            serial::Config::default(),
            clocks,
            &mut rcc.apb1,
        ));

        uart_serial.init();

        ps.set_ui_loading("sd_card");
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

        ps.set_ui_loading("buttons");
        display.render(&ps).unwrap();
        let btn_pause_pin = gpiob.pb0.into_pull_up_input(&mut gpiob.crl);
        let btn_pause = Button::new(btn_pause_pin, &device.EXTI, &mut afio).unwrap();

        let btn_encoder_pin = gpioa.pa10.into_pull_up_input(&mut gpioa.crh);
        let btn_encoder = Button::new(btn_encoder_pin, &device.EXTI, &mut afio).unwrap();

        ps.set_ui_loading("rotary encoder");
        display.render(&ps).unwrap();

        gpioa.pa8.into_pull_up_input(&mut gpioa.crh);
        gpioa.pa9.into_pull_up_input(&mut gpioa.crh);
        let rotary_encoder = RotaryEncoder::new(device.TIM1);

        ps.set_ui_loading("resources");
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("resources"));

        let uart_rx_buf = Vec::new();
        let usb_rx_buf = Vec::new();

        init::LateResources {
            ps,
            led,
            usb_serial,
            uart_serial,
            display,
            sdcard,
            uart_rx_buf,
            usb_rx_buf,
            query: None,
            query_idx: 0,

            btn_pause,
            btn_encoder,
            rotary_encoder,
        }
    }

    #[idle(resources = [
        ps,
        led,
        display,
        sdcard,
        usb_serial,
        uart_serial,
        usb_rx_buf,
        uart_rx_buf,
        query,
        btn_pause,
        btn_encoder,
        rotary_encoder,
    ])]
    fn idle(cx: idle::Context) -> ! {
        let mut il = IdleLoop::new(cx);
        il.boot().unwrap();

        loop {
            il.try_read_lines();
            il.handle_state_ok();
        }
    }

    #[task(resources = [query, query_idx],
           schedule = [ping],
           priority = 1)]
    fn ping(cx: ping::Context) {
        let q = cx.resources.query;
        let qidx = cx.resources.query_idx;
        if q.is_none() {
            q.replace(QUERY_PING_LOOP[*qidx]);
            *qidx = (*qidx + 1usize) % QUERY_PING_LOOP.len();
        }

        cx.schedule
            .ping(cx.scheduled + Duration::from_cycles(SYS_FREQ.0 / 16))
            .unwrap();
    }

    #[task(binds = EXTI0,
        resources = [btn_pause],
        priority = 2)]
    fn btn_pause_poll(cx: btn_pause_poll::Context) {
        cx.resources.btn_pause.poll().unwrap();
    }

    #[task(binds = EXTI15_10,
        resources = [btn_encoder],
        priority = 2)]
    fn btn_encoder_poll(cx: btn_encoder_poll::Context) {
        cx.resources.btn_encoder.poll().unwrap();
    }

    #[task(binds = USART2,
        resources = [uart_serial, uart_rx_buf],
        priority = 3)]
    fn uart_poll(cx: uart_poll::Context) {
        let uart_serial = cx.resources.uart_serial;
        let mut uart_rx_buf = cx.resources.uart_rx_buf;
        uart_serial.fill_buf(&mut uart_rx_buf).unwrap();
    }

    #[task(binds = USB_HP_CAN_TX,
            resources = [usb_serial],
            priority = 4)]
    fn usb_tx(cx: usb_tx::Context) {
        cx.resources.usb_serial.poll();
    }

    #[task(binds = USB_LP_CAN_RX0,
            resources = [usb_serial, usb_rx_buf],
            priority = 4)]
    fn usb_rx(cx: usb_rx::Context) {
        let usb_serial = cx.resources.usb_serial;
        let mut usb_rx_buf = cx.resources.usb_rx_buf;
        usb_serial.read(&mut usb_rx_buf).unwrap();
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

struct IdleLoop<'a> {
    led: &'a mut LedPin,
    usb_serial: resources::usb_serial<'a>,
    usb_rx_buf: resources::usb_rx_buf<'a>,
    uart_serial: resources::uart_serial<'a>,
    uart_rx_buf: resources::uart_rx_buf<'a>,
    query: resources::query<'a>,
    query_sent: bool,
    ps: &'a mut PS,
    display: &'a mut Display,
    sdc: &'a mut SDCard,
    usb_line_buf: Vec<u8, U64>,
    usb_eol: bool,
    uart_line_buf: Vec<u8, U64>,
    uart_eol: bool,

    btn_pause: resources::btn_pause<'a>,
    btn_encoder: resources::btn_encoder<'a>,
    rotary_encoder: &'a mut RotaryEncoder,
}

impl<'a> IdleLoop<'a> {
    pub fn new(cx: idle::Context<'a>) -> Self {
        IdleLoop {
            led: cx.resources.led,
            usb_serial: cx.resources.usb_serial,
            usb_rx_buf: cx.resources.usb_rx_buf,
            uart_serial: cx.resources.uart_serial,
            uart_rx_buf: cx.resources.uart_rx_buf,
            query: cx.resources.query,
            query_sent: false,
            ps: cx.resources.ps,
            display: cx.resources.display,
            sdc: cx.resources.sdcard,
            usb_line_buf: Vec::new(),
            usb_eol: false,
            uart_line_buf: Vec::new(),
            uart_eol: false,

            btn_pause: cx.resources.btn_pause,
            btn_encoder: cx.resources.btn_encoder,
            rotary_encoder: cx.resources.rotary_encoder,
        }
    }

    #[inline]
    fn render_loading(&mut self, s: &'static str) -> Result<(), AppError> {
        self.ps.set_ui_loading(s);
        self.display.render(self.ps)
    }

    #[inline]
    fn show_err<F>(&mut self, mut fun: F) -> Result<(), AppError>
    where
        F: FnMut(&mut Self) -> Result<(), AppError>,
    {
        match fun(self) {
            Err(e) => self.ps.show_error(e),
            _ => (),
        }

        Ok(())
    }

    #[inline]
    fn show_err_ok<F>(&mut self, fun: F)
    where
        F: FnMut(&mut Self) -> Result<(), AppError>,
    {
        self.show_err(fun).ok();
    }

    pub fn boot(&mut self) -> Result<(), AppError> {
        self.render_loading(".,.,.")?;

        // Give GPIB/serial interface time to boot
        for _ in 0..100 {
            self.usb_serial.lock(|s| s.poll());
            asm::delay(SYS_FREQ.0 / 100);
        }

        self.render_loading("BOOT")?;

        self.show_err_ok(|slf| {
            let sdc = &mut slf.sdc;
            // we won't receive anything while sending the whole file but that's Ok
            slf.uart_serial
                .lock(|us| sdc.send_file("BOOT", |buf| us.write_buf_flush(buf)))
        });

        self.drain_uart_rx(); // in case there's any junk from loading a boot file
        self.render_loading("DONE")?;

        self.ps.set_ui_info_screen();

        Ok(())
    }

    pub fn try_read_lines(&mut self) {
        self.usb_serial.lock(|s| s.poll());

        let usb_rx_buf = &mut self.usb_rx_buf;
        let usb_line_buf = &mut self.usb_line_buf;

        let uart_rx_buf = &mut self.uart_rx_buf;
        let uart_line_buf = &mut self.uart_line_buf;

        self.usb_eol = usb_rx_buf.lock(|b| fill_until_eol(usb_line_buf, b));

        self.uart_eol = uart_rx_buf.lock(|b| fill_until_eol(uart_line_buf, b));

        ifcfg!(
            "bin_debug",
            hprintln!(
                "loop {} {} {} {}",
                self.usb_line_buf.len(),
                self.usb_eol,
                uart_line_buf.len(),
                self.uart_eol
            )
        );

        // 1st line of input switches into UART serial adapter mode
        if self.usb_eol {
            self.ps.set_ui_usb_serial();
            // Read what's left in the UART and throw out.
            // USB serial will drive IO now
            self.drain_uart_rx();
        }
    }

    /// Read/throw away what's currently in the buffer
    fn drain_uart_rx(&mut self) {
        let uart_line_buf = &mut self.uart_line_buf;
        uart_line_buf.clear();
        for _ in 0..4 {
            asm::delay(SYS_FREQ.0 / 8);
            self.uart_rx_buf.lock(|b| fill_until_eol(uart_line_buf, b));
            uart_line_buf.clear();
        }
        self.uart_eol = false;
    }

    /// Main loop, ignore errors or crash completely on this level
    pub fn handle_state_ok(&mut self) {
        self.led.toggle().unwrap();
        self.show_err_ok(|slf| slf.handle_state());
        self.display.render(self.ps).unwrap();
    }

    #[inline]
    fn handle_state(&mut self) -> Result<(), AppError> {
        let encoder_change = self.rotary_encoder.poll();
        match &mut self.ps.ui {
            UI::USSBSerial => self.handle_state_usb_serial(),
            UI::InfoScreen(is) => IdleLoop::handle_state_info_screen(
                encoder_change,
                &mut self.btn_pause,
                &mut self.btn_encoder,
                &mut self.usb_serial,
                &mut self.uart_serial,
                &mut self.uart_eol,
                &mut self.uart_line_buf,
                &mut self.query,
                &mut self.query_sent,
                is,
            ),
            _ => Ok(()),
        }
    }

    #[inline]
    fn handle_state_usb_serial(&mut self) -> Result<(), AppError> {
        let usb_line_buf = &mut self.usb_line_buf;
        let uart_line_buf = &mut self.uart_line_buf;

        // Act as a UART serial adapter until reboot
        if self.uart_eol {
            self.usb_serial.lock(|s| s.write(&uart_line_buf))?;
            self.uart_line_buf.clear();
        }

        // Lock means we can't receive while writing but it's Ok
        // for this particular request/response protocol
        if self.usb_eol {
            self.uart_serial
                .lock(|s| s.write_buf_flush(&usb_line_buf))?;
            self.usb_line_buf.clear();
        }

        Ok(())
    }

    #[inline]
    fn handle_state_info_screen(
        encoder_change: i16,
        btn_pause: &mut resources::btn_pause<'a>,
        btn_encoder: &mut resources::btn_encoder<'a>,
        usb_serial: &mut resources::usb_serial<'a>,
        uart_serial: &mut resources::uart_serial<'a>,
        uart_eol: &mut bool,
        uart_line_buf: &mut Vec<u8, U64>,
        query: &mut resources::query<'a>,
        query_sent: &mut bool,
        is: &mut InfoScreen,
    ) -> Result<(), AppError> {
        let pause_press = btn_pause.lock(|b| b.take_last_press(time::MilliSeconds(60)));
        let encoder_press = btn_encoder.lock(|b| b.take_last_press(time::MilliSeconds(60)));

        ifcfg!("bin_info", {
            (match pause_press {
                None => Ok(()),
                Some(p) => hprintln!("BTN P {}", p.0),
            })
            .and_then(|_| match encoder_press {
                None => Ok(()),
                Some(e) => hprintln!("BTN E {}", e.0),
            })
            .and_then(|_| {
                if encoder_change != 0 {
                    hprintln!("RE {}", encoder_change)
                } else {
                    Ok(())
                }
            })
        });

        let q = query.lock(|qopt| match qopt {
            None => Ok::<Option<Query>, AppError>(None),
            Some(q) => {
                if !(*query_sent) {
                    let mut sbuf: String<U32> = String::new();
                    q.write_serial_cmd_buf(&mut sbuf);
                    uart_serial.lock(|s| s.write_buf_flush(&sbuf.into_bytes()))?;
                    *query_sent = true;
                }

                if *uart_eol {
                    *query_sent = false;
                    Ok(qopt.take())
                } else {
                    Ok(None)
                }
            }
        });

        match q? {
            None => {}
            Some(q) => {
                let v = parse_f32(uart_line_buf)?;
                uart_line_buf.clear();

                ifcfg!("bin_info", hprintln!("qres {:?} {}", q, v));
                is.set_query_result(&q, v);
                // send query/response to USB host
                let mut buf: String<U64> = String::new();
                buf.push_str(&q.to_str()).map_err(|_| AppError::Duh)?;
                write!(buf, "\t{:.3}\r\n", v).map_err(|_| AppError::Duh)?;
                usb_serial.lock(|s| s.write(&buf.into_bytes()))?;
            }
        }
        Ok(())
    }
}
