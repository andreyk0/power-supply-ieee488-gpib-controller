use core::{convert::Infallible, fmt::Write};

use embedded_hal::blocking::delay::DelayUs;

use embedded_graphics::{
    egtext, fonts::*, pixelcolor::BinaryColor, prelude::*, primitives::*, style::*, text_style,
};

use stm32f4xx_hal::spi;

use heapless::{consts::*, String};

use crate::{delay::*, model::*, prelude::*};

// 0 to n-1 based
pub const WIDTH: i32 = 127;
pub const HEIGHT: i32 = 63;

const FILES_PER_SCREEN: usize = 8;

pub struct Display {
    device: DisplayDevice,
}

impl Display {
    pub fn new(mut device: DisplayDevice) -> Result<Self, AppError> {
        let mut delay = AsmDelay {};
        device.init(&mut delay)?;
        device.clear(&mut delay)?;
        Ok(Display { device })
    }

    pub fn clear(self: &mut Self) -> Result<(), AppError> {
        Rectangle::new(Point::zero(), Point::new(WIDTH, HEIGHT))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(BinaryColor::Off)
                    .build(),
            )
            .draw(&mut self.device)?;

        Ok(())
    }

    pub fn flush(self: &mut Self) -> Result<(), AppError> {
        let mut delay = AsmDelay {};
        self.device.flush(&mut delay)?;
        Ok(())
    }

    pub fn render(self: &mut Self, ps: &PS) -> Result<(), AppError> {
        self.clear()?;

        match &ps.error {
            Some(e) => self.render_error(&e)?,
            None => self.render_ui(&ps.ui)?,
        }

        self.flush()?;

        ifcfg!("render_debug", self.debug_delay());
        Ok(())
    }

    fn render_error(self: &mut Self, e: &AppError) -> Result<(), AppError> {
        let mut s: String<U32> = String::new();
        write!(&mut s, "{:?}", e)?;

        egtext!(
            text = &s,
            top_left = Point::new(2, HEIGHT / 2 - 3),
            style = text_style!(font = Font6x6, text_color = BinaryColor::On,)
        )
        .draw(&mut self.device)?;

        Ok(())
    }

    #[inline]
    fn render_ui(self: &mut Self, ps: &UI) -> Result<(), AppError> {
        match ps {
            UI::UILoading(s) => self.render_ui_loading(s),
            UI::USSBSerial => self.render_usb_serial(),
            UI::InfoScreen(is) => self.render_info_screen(is),
            UI::ProjectFiles(pfs) => self.render_project_files(pfs),
        }
    }

    #[inline]
    fn render_ui_loading(self: &mut Self, s: &str) -> Result<(), AppError> {
        let mut buf: String<U64> = String::new();
        write!(&mut buf, "Loading {} ...", s)?;

        egtext!(
            text = &buf,
            top_left = Point::new(2, HEIGHT / 2 - 3),
            style = text_style!(font = Font6x6, text_color = BinaryColor::On,)
        )
        .draw(&mut self.device)?;

        Ok(())
    }

    #[inline]
    fn render_usb_serial(self: &mut Self) -> Result<(), AppError> {
        egtext!(
            text = "<< USB serial >>",
            top_left = Point::new(2, HEIGHT / 2 - 3),
            style = text_style!(font = Font6x6, text_color = BinaryColor::On,)
        )
        .draw(&mut self.device)?;

        Ok(())
    }

    #[inline]
    fn render_info_screen(self: &mut Self, info: &InfoScreen) -> Result<(), AppError> {
        self.render_ps_channel(
            0,
            &info.ch1,
            info.uich.as_ref().map(|u| &u.ch1),
            &info.vsel,
            info.chsel.is_selected(ChSelected::Ch1),
            false,
        )?;

        self.render_ps_channel(
            74,
            &info.ch2,
            info.uich.as_ref().map(|u| &u.ch2),
            &info.vsel,
            info.chsel.is_selected(ChSelected::Ch2),
            true,
        )?;

        Ok(())
    }

    fn render_ps_channel(
        self: &mut Self,
        xoff: i32,
        ch: &PSChannel,
        uich: Option<&UIChannel>,
        vsel: &VarSelected,
        chsel: bool,
        dash_graph: bool,
    ) -> Result<(), AppError> {
        let mut s: String<U32> = String::new();
        let mut iselstr = "=";
        let mut vselstr = "=";

        if chsel {
            match vsel {
                VarSelected::V => vselstr = "*",
                VarSelected::I => iselstr = "*",
            }
        }

        write!(s, "{:6.3} V ", OptF32Fmt(ch.vout))?;

        egtext!(
            text = &s,
            top_left = Point::new(xoff, 0),
            style = text_style!(
                font = Font6x8,
                text_color = BinaryColor::Off,
                background_color = BinaryColor::On
            )
        )
        .draw(&mut self.device)?;

        s.clear();
        write!(s, "{:6.3} A ", OptF32Fmt(ch.iout))?;

        egtext!(
            text = &s,
            top_left = Point::new(xoff, 8),
            style = text_style!(
                font = Font6x8,
                text_color = BinaryColor::Off,
                background_color = BinaryColor::On
            )
        )
        .draw(&mut self.device)?;

        s.clear();
        write!(s, "{:6.3} W ", OptF32Fmt(ch.pout()))?;

        egtext!(
            text = &s,
            top_left = Point::new(xoff, 16),
            style = text_style!(
                font = Font6x8,
                text_color = BinaryColor::Off,
                background_color = BinaryColor::On
            )
        )
        .draw(&mut self.device)?;

        s.clear();
        write!(
            s,
            "V{} {:6.3}",
            vselstr,
            OptF32Fmt(uich.map(|u| u.vset).or(ch.vset)),
        )?;

        egtext!(
            text = &s,
            top_left = Point::new(xoff, 25),
            style = text_style!(font = Font6x8, text_color = BinaryColor::On,)
        )
        .draw(&mut self.device)?;

        s.clear();
        write!(
            s,
            "I{} {:6.3}",
            iselstr,
            OptF32Fmt(uich.map(|u| u.iset).or(ch.iset)),
        )?;

        egtext!(
            text = &s,
            top_left = Point::new(xoff, 33),
            style = text_style!(font = Font6x8, text_color = BinaryColor::On,)
        )
        .draw(&mut self.device)?;

        match ch.relative_power_samples_itr() {
            None => (),
            Some(ps) => {
                let mut x = 0;
                for p in ps {
                    let py = ((p * 22.0) as i32).max(0).min(22);
                    let c = if dash_graph {
                        if x % 2 == 0 {
                            BinaryColor::On
                        } else {
                            BinaryColor::Off
                        }
                    } else {
                        BinaryColor::On
                    };
                    Pixel(Point::new(x, HEIGHT - py), c).draw(&mut self.device)?;
                    x += 1;
                }
            }
        }

        Ok(())
    }

    #[inline]
    fn render_project_files(self: &mut Self, pfs: &ProjectFiles) -> Result<(), AppError> {
        if pfs.fnames.is_empty() {
            egtext!(
                text = "<<< No files >>>",
                top_left = Point::new(2, HEIGHT / 2 - 3),
                style = text_style!(font = Font6x6, text_color = BinaryColor::On,)
            )
            .draw(&mut self.device)?;
        } else {
            let page_num = pfs.selected / FILES_PER_SCREEN;
            let begin = page_num * FILES_PER_SCREEN;
            let end = (begin + FILES_PER_SCREEN).min(pfs.fnames.len());
            self.render_project_files_page(
                pfs.selected % FILES_PER_SCREEN,
                &pfs.fnames[begin..end],
            )?;
        }

        Ok(())
    }

    #[inline]
    fn render_project_files_page(
        self: &mut Self,
        selected: usize,
        fnames: &[String<U32>],
    ) -> Result<(), AppError> {
        let mut voffset = 2;
        let mut idx = 0;

        let p1 = Point::new(0, 0);
        let p2 = Point::new(3, 3);
        let p3 = Point::new(0, 6);
        let cursor = Triangle::from_points([p1, p2, p3]);

        for fname in fnames {
            egtext!(
                text = fname.as_str(),
                top_left = Point::new(9, voffset),
                style = text_style!(font = Font6x6, text_color = BinaryColor::On,)
            )
            .draw(&mut self.device)?;

            if idx == selected {
                cursor
                    .translate(Point::new(3, voffset))
                    .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                    .draw(&mut self.device)?;
            }

            idx += 1;
            voffset += 7;

            ifcfg!("render_debug", self.debug_delay());
        }

        Ok(())
    }

    #[inline]
    fn debug_delay(&mut self) -> Result<(), AppError> {
        let mut delay = AsmDelay {};
        self.flush()?;
        delay.delay_us(1000000);
        Ok(())
    }
}

impl From<st7920::Error<spi::Error, Infallible>> for AppError {
    fn from(_: st7920::Error<spi::Error, Infallible>) -> Self {
        AppError::DisplayError("SPI")
    }
}

struct OptF32Fmt(Option<f32>);

impl core::fmt::Display for OptF32Fmt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            None => f.write_str("---"),
            Some(v) => {
                if v < 0.0f32 {
                    0.0f32.fmt(f)
                } else {
                    v.fmt(f)
                }
            }
        }
    }
}
