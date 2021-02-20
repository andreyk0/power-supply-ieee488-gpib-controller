#![deny(unsafe_code)]
#![no_std]

#[macro_use]
pub mod macros;

pub mod consts;
pub mod delay;
pub mod display;
pub mod error;
pub mod line;
pub mod model;
pub mod protocol;
pub mod sdcard;
pub mod time;
pub mod types;
pub mod uart_serial;
pub mod usb_serial;

pub mod prelude {
    pub use crate::consts::*;
    pub use crate::error::*;
    pub use crate::types::*;
}
