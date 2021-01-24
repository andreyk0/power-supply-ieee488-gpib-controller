use cortex_m::asm;
use cortex_m_semihosting::dbg;

use stm32f1xx_hal::{gpio, i2c, prelude::*, timer, usb};

use embedded_hal::digital::v2::OutputPin;

use rtic::cyccnt::Duration;

use usb_device::{bus, prelude::*};
use usbd_serial::{DefaultBufferStore, Result, SerialPort};

use crate::{consts::*, types::*};

use core::borrow::BorrowMut;

pub struct UsbSerial<'a, B>
where
    B: bus::UsbBus,
{
    serial_port: SerialPort<'a, B, DefaultBufferStore, DefaultBufferStore>,
    usb_device: UsbDevice<'a, B>,
}

impl<B> UsbSerial<'_, B>
where
    B: bus::UsbBus,
{
    /// New usb serial
    pub fn new<'a>(usb_bus: &'a bus::UsbBusAllocator<B>) -> UsbSerial<'a, B> {
        let usb_device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("DIY")
            .product("power-supply-ieee488-gpib-controller")
            .serial_number("1")
            .device_class(usbd_serial::USB_CLASS_CDC)
            .build();

        let serial_port = SerialPort::new(&usb_bus);

        UsbSerial {
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

    /// Serial read
    #[inline]
    pub fn read(&mut self, data: &mut [u8]) -> Result<usize> {
        self.serial_port.read(data)
    }

    /// Serial write
    #[inline]
    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.serial_port.write(data)
    }
}
