use st7920::ST7920;
use stm32f1xx_hal::gpio::*;
use stm32f1xx_hal::{gpio, pac, serial, spi, usb};

use crate::{time::*, usb_serial};

pub type LedPin = gpioc::PC13<Output<PushPull>>;

pub type UsbSerial = usb_serial::UsbSerialDevice<'static, usb::UsbBus<usb::Peripheral>>;

pub type UartSerialDevice = serial::Serial<
    pac::USART2,
    (
        gpio::gpioa::PA2<gpio::Alternate<gpio::PushPull>>,
        gpio::gpioa::PA3<gpio::Input<gpio::Floating>>,
    ),
>;

pub type DisplayDevice = ST7920<
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
    gpio::gpioa::PA6<gpio::Output<gpio::PushPull>>,
    gpio::gpioa::PA4<gpio::Output<gpio::PushPull>>,
>;

pub type SDCardController = embedded_sdmmc::Controller<
    embedded_sdmmc::SdMmcSpi<
        spi::Spi<
            stm32f1xx_hal::pac::SPI2,
            spi::Spi2NoRemap,
            (
                gpio::gpiob::PB13<gpio::Alternate<gpio::PushPull>>,
                gpio::gpiob::PB14<gpio::Input<gpio::Floating>>,
                gpio::gpiob::PB15<gpio::Alternate<gpio::PushPull>>,
            ),
            u8,
        >,
        gpio::gpiob::PB12<gpio::Output<gpio::PushPull>>,
    >,
    DummyTimeSource,
>;

pub type PauseButtonPin = gpiob::PB0<Input<PullUp>>;

pub type EncoderButtonPin = gpioa::PA10<Input<PullUp>>;
