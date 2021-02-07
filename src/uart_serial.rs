use stm32f1xx_hal::{prelude::*, serial::*};

use heapless::{ArrayLength, Vec};

use crate::types::*;

pub struct UartSerial {
    device: UartSerialDevice,
}

impl UartSerial {
    pub fn new(device: UartSerialDevice) -> Self {
        UartSerial { device }
    }

    pub fn init(&mut self) {
        self.device.listen(Event::Rxne);
    }

    pub fn try_flush(&mut self) {
        self.device.flush().map_or((), |_| ())
    }

    pub fn try_write_buf(&mut self, buf: &[u8]) {
        for c in buf {
            self.device.write(*c).map_or((), |_| ())
        }
    }

    pub fn try_fill_buf<S>(&mut self, buf: &mut Vec<u8, S>)
    where
        S: ArrayLength<u8>,
    {
        while self
            .device
            .read()
            .map_or(false, |c| buf.push(c).map_or(false, |_| true))
        { /**/ }
    }
}
