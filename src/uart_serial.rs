use cortex_m_semihosting::*;

use nb;

use stm32f1xx_hal::{prelude::*, serial::*};

use heapless::{ArrayLength, Vec};

use crate::prelude::*;

pub struct UartSerial {
    device: UartSerialDevice,
}

impl UartSerial {
    pub fn new(device: UartSerialDevice) -> Self {
        UartSerial { device }
    }

    pub fn init(&mut self) {
        self.device.listen(Event::Rxne)
    }

    pub fn flush(&mut self) -> Result<(), AppError> {
        ifcfg!("uart_debug", hprintln!("UART flush"));
        nb::block!(self.device.flush())?;
        Ok(())
    }

    pub fn write_buf(&mut self, buf: &[u8]) -> Result<(), AppError> {
        for c in buf {
            ifcfg!("uart_debug", hprintln!("UART write {}", c));
            nb::block!(self.device.write(*c))?;
        }
        Ok(())
    }

    pub fn fill_buf<S>(&mut self, buf: &mut Vec<u8, S>) -> Result<(), AppError>
    where
        S: ArrayLength<u8>,
    {
        while let (Ok(b), true) = (self.device.read(), buf.len() < buf.capacity()) {
            buf.push(b).map_err(|_| AppError::Duh)?
        }
        Ok(())
    }
}

impl From<stm32f1xx_hal::serial::Error> for AppError {
    fn from(_: stm32f1xx_hal::serial::Error) -> Self {
        AppError::UartSerialError
    }
}
