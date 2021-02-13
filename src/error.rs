//! Application errors

use core::convert::Infallible;

/// User visible errors
#[derive(Debug)]
pub enum AppError {
    Duh,
    FmtError,
    InitError(&'static str),
    DisplayError(&'static str),
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
