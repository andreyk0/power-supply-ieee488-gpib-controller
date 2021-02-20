use core::{convert::Infallible, fmt::Write};

use embedded_hal::blocking::delay::DelayUs;

use embedded_graphics::{
    egtext, fonts::*, pixelcolor::BinaryColor, prelude::*, primitives::*, style::*, text_style,
};

use stm32f1xx_hal::spi;

use heapless::{consts::*, String};

use crate::{delay::*, model::*, prelude::*};

// 0 to n-1 based
pub const WIDTH: i32 = 127;
pub const HEIGHT: i32 = 63;

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
                    .stroke_width(1)
                    .stroke_color(BinaryColor::On)
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

        ifcfg!("render_debug", {
            let mut delay = AsmDelay {};
            delay.delay_us(1000000);
            Ok::<(), ()>(())
        });
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
        self.render_ps_channel(2, &info.ch1)?;
        self.render_ps_channel(65, &info.ch2)
    }

    fn render_ps_channel(self: &mut Self, xoff: i32, ch: &PSChannel) -> Result<(), AppError> {
        let mut s: String<U32> = String::new();

        write!(s, "V: {:.3}", OptF32Fmt(ch.vout))?;

        egtext!(
            text = &s,
            top_left = Point::new(xoff + 2, 5),
            style = text_style!(font = Font6x8, text_color = BinaryColor::On,)
        )
        .draw(&mut self.device)?;

        s.clear();
        write!(s, "I: {:.3}", OptF32Fmt(ch.iout))?;

        egtext!(
            text = &s,
            top_left = Point::new(xoff + 2, 14),
            style = text_style!(font = Font6x8, text_color = BinaryColor::On,)
        )
        .draw(&mut self.device)?;

        s.clear();
        write!(s, "Vset: {:.3}", OptF32Fmt(ch.vset))?;

        egtext!(
            text = &s,
            top_left = Point::new(xoff + 2, 23),
            style = text_style!(font = Font6x6, text_color = BinaryColor::On,)
        )
        .draw(&mut self.device)?;

        s.clear();
        write!(s, "Iset: {:.3}", OptF32Fmt(ch.iset))?;

        egtext!(
            text = &s,
            top_left = Point::new(xoff + 2, 30),
            style = text_style!(font = Font6x6, text_color = BinaryColor::On,)
        )
        .draw(&mut self.device)?;

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
