use cortex_m_semihosting::*;

use heapless::{ArrayLength, Vec};

use usb_device::{bus, prelude::*, UsbError::WouldBlock};
use usbd_serial::{DefaultBufferStore, SerialPort};

use crate::prelude::*;

pub struct UsbSerialDevice<'a, B>
where
    B: bus::UsbBus,
{
    serial_port: SerialPort<'a, B, DefaultBufferStore, DefaultBufferStore>,
    usb_device: UsbDevice<'a, B>,
}

impl<B> UsbSerialDevice<'_, B>
where
    B: bus::UsbBus,
{
    /// New usb serial
    pub fn new<'a>(usb_bus: &'a bus::UsbBusAllocator<B>) -> UsbSerialDevice<'a, B> {
        // this has to go before UsbDeviceBuilder, it mutably borrows from
        // refcells but doesn't exit scope and anything else trying to do
        // the same panics in refcell's borrow mut call
        let serial_port = SerialPort::new(&usb_bus);

        let usb_device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("DIY")
            .product("PS-GPIB")
            .serial_number("1")
            .device_class(usbd_serial::USB_CLASS_CDC)
            .build();

        UsbSerialDevice {
            serial_port,
            usb_device,
        }
    }

    /// Poll periodically
    #[inline]
    pub fn poll(&mut self) {
        if !self.usb_device.poll(&mut [&mut self.serial_port]) {
            // https://github.com/mvirkkunen/usb-device/issues/32
            usb_device::class::UsbClass::poll(&mut self.serial_port);
        }
    }

    /// Serial read append to the given vector, non-blocking
    #[inline]
    pub fn read<S>(&mut self, data: &mut Vec<u8, S>) -> Result<(), AppError>
    where
        S: ArrayLength<u8>,
    {
        self.poll();

        let mut buf = [0u8; 1];
        while self.serial_port.dtr() && (data.capacity() > data.len()) {
            match self.serial_port.read(&mut buf) {
                Ok(_) => {
                    ifcfg!("usb_debug", hprintln!("USB read {}", buf[0]));
                    data.push(buf[0]).map_err(|_| AppError::UsbSerialError)?;
                }
                Err(WouldBlock) => break,
                e => e.map(|_| ())?,
            }

            self.poll();
        }

        Ok(())
    }

    /// Serial write all bytes out, blocking
    #[inline]
    pub fn write(&mut self, data: &[u8]) -> Result<(), AppError> {
        self.poll();

        if self.serial_port.dtr() {
            let mut n = 0;
            while n < data.len() - 1 {
                match self.serial_port.write(&data[n..]) {
                    Ok(s) => {
                        ifcfg!("usb_debug", hprintln!("USB write {}", s));
                        n += s;
                    }
                    Err(WouldBlock) => {
                        ifcfg!("usb_debug", hprintln!("USB write blocked"));
                    }
                    e => {
                        e?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl From<UsbError> for AppError {
    fn from(_: UsbError) -> Self {
        AppError::UsbSerialError
    }
}
