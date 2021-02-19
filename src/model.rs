//! UI model

use crate::error::*;

/// State of the power supply controller
pub struct PS {
    pub error: Option<AppError>,
    pub ui: UI,
}

pub struct InfoScreen {}

/// UI states
pub enum UI {
    UILoading(&'static str),
    USSBSerial,
    InfoScreen(InfoScreen),
}

impl PS {
    pub fn new() -> Self {
        PS {
            error: None,
            ui: UI::UILoading("Initializing..."),
        }
    }

    #[inline]
    pub fn show_error(&mut self, e: AppError) {
        if self.error.is_none() {
            self.error = Some(e)
        }
    }

    #[inline]
    pub fn clear_error(&mut self) {
        self.error = None
    }

    #[inline]
    pub fn set_ui_loading(&mut self, s: &'static str) {
        self.ui = UI::UILoading(s)
    }

    #[inline]
    pub fn set_ui_usb_serial(&mut self) {
        self.ui = UI::USSBSerial
    }

    #[inline]
    pub fn set_ui_info_screen(&mut self) {
        self.ui = UI::InfoScreen(InfoScreen {})
    }
}
