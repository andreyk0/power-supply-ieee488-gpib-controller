use embedded_graphics::{
    fonts::{Font6x8, Text},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Circle,
    style::{PrimitiveStyle, TextStyle},
};

use crate::{delay::*, types::*};

pub struct Display {
    device: DisplayDevice,
}

impl Display {
    pub fn new(mut device: DisplayDevice) -> Display {
        let mut delay = AsmDelay {};

        device.init(&mut delay).unwrap();
        device.clear(&mut delay).unwrap();

        Display { device }
    }

    pub fn test(self: &mut Self) {
        let mut delay = AsmDelay {};

        let c = Circle::new(Point::new(20, 20), 8)
            .into_styled(PrimitiveStyle::with_fill(BinaryColor::On));
        let t = Text::new("Moohahaha!", Point::new(40, 16))
            .into_styled(TextStyle::new(Font6x8, BinaryColor::On));

        c.draw(&mut self.device).unwrap();
        t.draw(&mut self.device).unwrap();

        self.device
            .flush(&mut delay)
            .expect("could not flush display");
    }
}
