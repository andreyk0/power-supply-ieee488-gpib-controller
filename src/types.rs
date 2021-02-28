use st7920::ST7920;
use stm32f4xx_hal::gpio::*;
use stm32f4xx_hal::{gpio, otg_fs, serial, spi, stm32};

use crate::{time::*, usb_serial};

pub type LedPin = gpioc::PC13<Output<PushPull>>;

pub type UsbSerial = usb_serial::UsbSerialDevice<'static, otg_fs::UsbBus<otg_fs::USB>>;

pub type UartSerialDevice = serial::Serial<
    stm32::USART1,
    (
        gpio::gpioa::PA15<gpio::Alternate<gpio::AF7>>,
        gpio::gpioa::PA10<gpio::Alternate<gpio::AF7>>,
    ),
>;

// ST7920<SPI, RST, CS>
pub type DisplayDevice = ST7920<
    spi::Spi<
        stm32::SPI1,
        (
            gpio::gpioa::PA5<gpio::Alternate<gpio::AF5>>,
            spi::NoMiso,
            gpio::gpioa::PA7<gpio::Alternate<gpio::AF5>>,
        ),
    >,
    gpio::gpioa::PA6<gpio::Output<gpio::PushPull>>,
    gpio::gpioa::PA4<gpio::Output<gpio::PushPull>>,
>;

pub type SDCardSPI = embedded_sdmmc::SdMmcSpi<
    spi::Spi<
        stm32::SPI2,
        (
            gpio::gpiob::PB13<gpio::Alternate<gpio::AF5>>,
            gpio::gpiob::PB14<gpio::Alternate<gpio::AF5>>,
            gpio::gpiob::PB15<gpio::Alternate<gpio::AF5>>,
        ),
    >,
    gpio::gpiob::PB12<gpio::Output<gpio::PushPull>>,
>;

pub type SDCardController = embedded_sdmmc::Controller<SDCardSPI, DummyTimeSource>;

pub type PauseButtonPin = gpioa::PA1<Input<PullUp>>;

pub type EncoderButtonPin = gpioa::PA2<Input<PullUp>>;
