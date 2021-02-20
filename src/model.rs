//! UI model

use crate::error::*;
use crate::protocol::*;

/// State of the power supply controller
pub struct PS {
    pub error: Option<AppError>,
    pub ui: UI,
}

// Single channel settings
pub struct PSChannel {
    pub vset: Option<f32>,
    pub vout: Option<f32>,
    pub iset: Option<f32>,
    pub iout: Option<f32>,
}

impl PSChannel {
    pub fn new() -> Self {
        PSChannel {
            vset: None,
            vout: None,
            iset: None,
            iout: None,
        }
    }

    #[inline]
    pub(super) fn set_query_result(&mut self, q: &Query, v: f32) {
        match q {
            Query::Vset(_) => self.vset = Some(v),
            Query::Iset(_) => self.iset = Some(v),
            Query::Vout(_) => self.vout = Some(v),
            Query::Iout(_) => self.iout = Some(v),
        }
    }
}

// Regular info screen, show current values
pub struct InfoScreen {
    pub ch1: PSChannel,
    pub ch2: PSChannel,
}

impl InfoScreen {
    pub fn new() -> Self {
        InfoScreen {
            ch1: PSChannel::new(),
            ch2: PSChannel::new(),
        }
    }

    pub fn set_query_result(&mut self, q: &Query, v: f32) {
        match q.query_channel() {
            None => {}
            Some(Channel::Ch1) => self.ch1.set_query_result(q, v),
            Some(Channel::Ch2) => self.ch2.set_query_result(q, v),
        }
    }
}

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
        self.ui = UI::InfoScreen(InfoScreen::new())
    }
}
