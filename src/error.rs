//! Application errors

use core::convert::Infallible;

/// User visible errors
#[derive(Debug, Clone, Copy)]
pub enum AppError {
    Duh,
    FmtError,
    SDError,
    UsbSerialError,
    UartSerialError,
    DisplayError(&'static str),
    ParseError,
    ProjectFileError,
}

impl From<Infallible> for AppError {
    fn from(_: Infallible) -> Self {
        AppError::Duh {}
    }
}

impl From<core::fmt::Error> for AppError {
    fn from(_: core::fmt::Error) -> Self {
        AppError::FmtError
    }
}
