#![cfg_attr(not(doc), no_main)]
#![no_std]

use panic_semihosting as _;
//TODO: use panic_halt as _;

use core::fmt::Write;

use cortex_m::asm;
use cortex_m_semihosting::*;

use stm32f4xx_hal::{
    otg_fs,
    prelude::*,
    serial, spi,
    time::{self, MilliSeconds},
};

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

// https://github.com/stm32-rs/stm32f4xx-hal/blob/master/examples/usb_serial.rs
static mut USB_EP_MEMORY: [u32; 1024] = [0; 1024];

#[rtic::app(device = stm32f4xx_hal::stm32,
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
    fn init(mut cx: init::Context) -> init::LateResources {
        static mut USB_BUS: Option<bus::UsbBusAllocator<otg_fs::UsbBus<otg_fs::USB>>> = None;

        let mut device = cx.device;
        let rcc = device.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(25.mhz())
            .sysclk(SYS_FREQ)
            .pclk1(48.mhz())
            .require_pll48clk() // USB
            .freeze();

        ifcfg!(
            "bin_debug",
            hprintln!(
                "clocks sysclk {} pclk1 {}",
                clocks.sysclk().0,
                clocks.pclk1().0
            )
        );

        ifcfg!("bin_info", hprintln!("gpio"));

        let gpioa = device.GPIOA.split();
        let gpiob = device.GPIOB.split();
        let gpioc = device.GPIOC.split();

        let mut ps = PS::new();

        ifcfg!("bin_info", hprintln!("display"));

        let lcd_sck = gpioa.pa5.into_alternate_af5();
        let lcd_mosi = gpioa.pa7.into_alternate_af5();
        let lcd_reset = gpioa.pa6.into_push_pull_output();
        let lcd_cs = gpioa.pa4.into_push_pull_output();

        let lcd_spi = spi::Spi::spi1(
            device.SPI1,
            (lcd_sck, spi::NoMiso, lcd_mosi),
            espi::MODE_0,
            time::Hertz(400_000),
            clocks,
        );

        let mut display =
            Display::new(ST7920::new(lcd_spi, lcd_reset, Some(lcd_cs), false)).unwrap();

        ps.set_ui_loading("ping");
        display.render(&ps).unwrap();

        let led = gpioc.pc13.into_push_pull_output();

        // Initialize (enable) the monotonic timer (CYCCNT)
        cx.core.DCB.enable_trace();
        // required on Cortex-M7 devices that software lock the DWT
        cortex_m::peripheral::DWT::unlock();
        cx.core.DWT.enable_cycle_counter();

        cx.schedule
            .ping(cx.start + Duration::from_cycles(SYS_FREQ.0 / 2))
            .unwrap();

        ps.set_ui_loading("usbp");
        display.render(&ps).unwrap();

        let usbp = otg_fs::USB {
            usb_global: device.OTG_FS_GLOBAL,
            usb_device: device.OTG_FS_DEVICE,
            usb_pwrclk: device.OTG_FS_PWRCLK,
            pin_dm: gpioa.pa11.into_alternate_af10(),
            pin_dp: gpioa.pa12.into_alternate_af10(),
        };

        ps.set_ui_loading("usb_bus");
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("usb_bus"));

        *USB_BUS = unsafe { Some(otg_fs::UsbBus::new(usbp, &mut USB_EP_MEMORY)) };
        let usb_bus = USB_BUS.as_ref().unwrap();

        ps.set_ui_loading("usb_serial");
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("usb_serial"));

        let usb_serial = UsbSerial::new(usb_bus);

        ps.set_ui_loading("uart_serial");
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("uart_serial"));

        let pin_tx = gpioa.pa15.into_alternate_af7();
        let pin_rx = gpioa.pa10.into_alternate_af7();

        let mut uart_serial = UartSerial::new(
            serial::Serial::usart1(
                device.USART1,
                (pin_tx, pin_rx),
                serial::config::Config::default().baudrate(115_200.bps()),
                clocks,
            )
            .unwrap(),
        );

        uart_serial.init();

        ps.set_ui_loading("sd_card");
        display.render(&ps).unwrap();
        ifcfg!("bin_info", hprintln!("sd_card"));

        let sd_sck = gpiob.pb13.into_alternate_af5();
        let sd_miso = gpiob.pb14.into_alternate_af5();
        let sd_mosi = gpiob.pb15.into_alternate_af5();
        let sd_cs = gpiob.pb12.into_push_pull_output();

        asm::delay(SYS_FREQ.0 / 4);

        let sd_spi = spi::Spi::spi2(
            device.SPI2,
            (sd_sck, sd_miso, sd_mosi),
            espi::MODE_0,
            time::Hertz(100_000),
            clocks,
        );

        let sdcard = SDCard::new(embedded_sdmmc::Controller::new(
            embedded_sdmmc::SdMmcSpi::new(sd_spi, sd_cs),
            DummyTimeSource {},
        ))
        .unwrap();

        ps.set_ui_loading("buttons");
        display.render(&ps).unwrap();
        let btn_pause_pin = gpioa.pa1.into_pull_up_input();
        let btn_pause = Button::new(btn_pause_pin, &mut device.EXTI, &mut device.SYSCFG).unwrap();

        let btn_encoder_pin = gpioa.pa2.into_pull_up_input();
        let btn_encoder =
            Button::new(btn_encoder_pin, &mut device.EXTI, &mut device.SYSCFG).unwrap();

        ps.set_ui_loading("rotary encoder");
        display.render(&ps).unwrap();

        // RM0383, Figure 17. Selecting an alternate function onSTM32F411xC/E
        gpioa.pa8.into_alternate_af1().internal_pull_up(true);
        gpioa.pa9.into_alternate_af1().internal_pull_up(true);
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

    #[task(binds = EXTI1,
            resources = [btn_pause],
            priority = 2)]
    fn btn_pause_poll(cx: btn_pause_poll::Context) {
        cx.resources.btn_pause.poll().unwrap();
    }

    #[task(binds = EXTI2,
            resources = [btn_encoder],
            priority = 2)]
    fn btn_encoder_poll(cx: btn_encoder_poll::Context) {
        cx.resources.btn_encoder.poll().unwrap();
    }

    #[task(binds = USART1,
            resources = [uart_serial, uart_rx_buf],
            priority = 3)]
    fn uart_poll(cx: uart_poll::Context) {
        let uart_serial = cx.resources.uart_serial;
        let mut uart_rx_buf = cx.resources.uart_rx_buf;
        uart_serial.fill_buf(&mut uart_rx_buf).unwrap();
    }

    #[task(binds = OTG_FS_WKUP,
                resources = [usb_serial, usb_rx_buf],
                priority = 4)]
    fn usb_fs_wkup(cx: usb_fs_wkup::Context) {
        cx.resources.usb_serial.poll();
    }

    #[task(binds = OTG_FS,
                resources = [usb_serial, usb_rx_buf],
                priority = 4)]
    fn usb_otg_fs(cx: usb_otg_fs::Context) {
        cx.resources.usb_serial.poll();
        let usb_serial = cx.resources.usb_serial;
        let mut usb_rx_buf = cx.resources.usb_rx_buf;
        usb_serial.read(&mut usb_rx_buf).unwrap();
    }

    // RTIC requires that unused interrupts are declared in an extern block when
    // using software tasks; these free interrupts will be used to dispatch the
    // software tasks.
    // Full list in stm32f4::stm32f411::Interrupt
    extern "C" {
        fn DMA2_STREAM0();
        fn DMA2_STREAM1();
        fn DMA2_STREAM2();
        fn DMA2_STREAM3();
        fn DMA2_STREAM4();
        fn I2C1_ER();
        fn I2C1_EV();
        fn I2C2_ER();
        fn I2C2_EV();
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

    next_command: String<U64>,

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
            next_command: String::new(),

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

    #[inline]
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
                .lock(|us| sdc.send_boot_file(|buf| us.write_buf_flush(buf)))
        });

        self.drain_uart_rx(); // in case there's any junk from loading a file
        self.render_loading("DONE")?;
        self.ps.set_ui_info_screen();
        Ok(())
    }

    pub fn load_project_file<'f>(&mut self, fname: &'f str) -> Result<(), AppError> {
        ifcfg!("bin_info", hprintln!("load_project_file {}", fname));

        self.drain_uart_rx(); // if there are any query results queued up

        self.render_loading(".,.,.")?;

        self.show_err_ok(|slf| {
            let sdc = &mut slf.sdc;
            // we won't receive anything while sending the whole file but that's Ok
            slf.uart_serial
                .lock(|us| sdc.send_file(fname, |buf| us.write_buf_flush(buf)))
        });

        self.drain_uart_rx(); // in case there's any junk from loading a file
        self.render_loading("DONE")?;

        self.ps.set_ui_info_screen();

        ifcfg!("bin_info", hprintln!("load_project_file DONE {}", fname));
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

        self.next_command.clear();
        self.uart_eol = false;
        self.query_sent = false;
        self.query.lock(|qopt| qopt.take());
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
        let button_press = self
            .btn_pause
            .lock(|b| b.take_last_press(time::MilliSeconds(60)));

        ifcfg!("bin_info", {
            match button_press {
                None => Ok(()),
                Some(p) => hprintln!("BTN P {}", p.0),
            }
        });

        match button_press {
            None => (),
            Some(pp) => {
                if pp > MilliSeconds(700) {
                    let pfs = ProjectFiles::new(self.sdc)?;
                    self.ps.ui = UI::ProjectFiles(pfs);
                }
            }
        }

        match &mut self.ps.ui {
            UI::UILoading(_) => Ok(()),
            UI::USSBSerial => self.handle_state_usb_serial(),
            UI::InfoScreen(is) => {
                match button_press {
                    None => (),
                    Some(pp) => {
                        if pp > MilliSeconds(100) {
                            self.next_command.clear(); // replace previous command
                            is.handle_on_off_button(&mut self.next_command)?;
                        }
                    }
                }

                is.ch1.sample_current_power();
                is.ch2.sample_current_power();

                IdleLoop::handle_state_info_screen(
                    encoder_change,
                    &mut self.btn_encoder,
                    &mut self.usb_serial,
                    &mut self.uart_serial,
                    &mut self.uart_eol,
                    &mut self.uart_line_buf,
                    &mut self.query,
                    &mut self.query_sent,
                    &mut self.next_command,
                    is,
                )
            }
            UI::ProjectFiles(pfs) => {
                let re_press_duration = self
                    .btn_encoder
                    .lock(|b| b.take_last_press(time::MilliSeconds(60)));
                match pfs.handle_rotary_encoder(re_press_duration, encoder_change)? {
                    Some(fname) => self.load_project_file(&fname),
                    None => Ok(()),
                }
            }
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
        btn_encoder: &mut resources::btn_encoder<'a>,
        usb_serial: &mut resources::usb_serial<'a>,
        uart_serial: &mut resources::uart_serial<'a>,
        uart_eol: &mut bool,
        uart_line_buf: &mut Vec<u8, U64>,
        query: &mut resources::query<'a>,
        query_sent: &mut bool,
        next_command: &mut String<U64>,
        is: &mut InfoScreen,
    ) -> Result<(), AppError> {
        let (encoder_press, btn_encoder_is_pressed) = btn_encoder.lock(|b| {
            if encoder_change != 0 {
                b.cancel_last_press()
            }
            (
                b.take_last_press(time::MilliSeconds(60)),
                b.is_pressed(time::MilliSeconds(30)),
            )
        });

        ifcfg!("bin_info", {
            (match encoder_press {
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

        // send latest command when there's no active query
        if (!(*query_sent)) && (!next_command.is_empty()) {
            unsafe {
                uart_serial.lock(|s| s.write_buf_flush(next_command.as_mut_vec()))?;
            }

            ifcfg!("bin_debug", hprintln!("sent {}", next_command));
            next_command.clear();
            asm::delay(SYS_FREQ.0 / 100);
        }

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
                let mut sbuf: String<U64> = String::new();
                to_str_skip_whitespace(uart_line_buf, &mut sbuf)?;
                uart_line_buf.clear();
                ifcfg!("bin_debug", hprintln!("qres {:?} {}", q, sbuf));

                is.set_query_result(&q, &sbuf)?;

                // send query/response to USB host
                let mut buf: String<U64> = String::new();
                buf.push_str(&q.to_str()).map_err(|_| AppError::Duh)?;
                write!(buf, "\t{}\r\n", sbuf).map_err(|_| AppError::Duh)?;
                usb_serial.lock(|s| s.write(&buf.into_bytes()))?;
            }
        }

        is.handle_rotary_encoder(
            encoder_press,
            btn_encoder_is_pressed,
            encoder_change,
            next_command,
        )?;

        Ok(())
    }
}
