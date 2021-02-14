//! UI model

use crate::error::*;

/// Action messages that facilitate state transitions
pub enum Act {
    ShowError(AppError),
    ClearError,
    UILoading(&'static str),
}

/// State of the power supply controller
pub struct PS {
    pub error: Option<AppError>,
    pub ui: UI,
}

/// UI states
pub enum UI {
    UILoading(&'static str),
}

impl PS {
    pub fn new() -> Self {
        PS {
            error: None,
            ui: UI::UILoading("Initializing..."),
        }
    }

    pub fn act(&mut self, a: Act) {
        match a {
            Act::ShowError(e) if self.error.is_none() => self.error = Some(e),
            Act::ShowError(_) => (),
            Act::ClearError => self.error = None,
            Act::UILoading(s) => self.ui = UI::UILoading(s),
        }
    }
}
