//! UI model

use num_traits::float::FloatCore;

use heapless::{consts::*, ArrayLength, String, Vec};

use rtic::cyccnt::Instant;
use stm32f4xx_hal::time::MilliSeconds;

use crate::{consts::SYS_FREQ, error::*, line::parse_str, protocol::*, sdcard::*};

// Single channel settings
pub struct PSChannel {
    pub vset: Option<f32>,
    pub vout: Option<f32>,
    pub iset: Option<f32>,
    pub iout: Option<f32>,
    pub out: Option<bool>,
}

impl PSChannel {
    pub fn new() -> Self {
        PSChannel {
            vset: None,
            vout: None,
            iset: None,
            iout: None,
            out: None,
        }
    }

    #[inline]
    pub(super) fn set_query_result<S>(&mut self, q: &Query, s: &String<S>) -> Result<(), AppError>
    where
        S: ArrayLength<u8>,
    {
        match q.header {
            ChannelHeader::Vset => self.vset = Some(parse_str(s)?),
            ChannelHeader::Iset => self.iset = Some(parse_str(s)?),
            ChannelHeader::Vout => self.vout = Some(parse_str(s)?),
            ChannelHeader::Iout => self.iout = Some(parse_str(s)?),
            ChannelHeader::Out => {
                self.out = Some({
                    let i: u32 = parse_str(s)?;
                    i != 0
                })
            }
        }

        Ok(())
    }
}

pub struct UIChannel {
    pub vset: f32,
    pub iset: f32,
}

impl UIChannel {
    pub fn new(v: f32, i: f32) -> Self {
        UIChannel {
            vset: (v * 10.0).round() / 10.0,
            iset: (i * 10.0).round() / 10.0,
        }
    }

    pub fn fix_range(&mut self) {
        self.vset = self.vset.min(20.0).max(0.0);

        self.iset = self
            .iset
            .min(if self.vset > 7.0 { 4.0 } else { 10.0 })
            .max(0.0);
    }
}

/// Keep channel state while rotary encoder is turning,
/// clear it out after a timeout and use query output.
/// (set/query turnaround is slow over serial link)
pub struct UIChannels {
    pub ch1: UIChannel,
    pub ch2: UIChannel,
    last_change: Instant,
}

impl UIChannels {
    #[inline]
    pub fn fix_range(&mut self) {
        self.ch1.fix_range();
        self.ch2.fix_range();
    }

    #[inline]
    fn iset_cmds<S>(&self, cmdbuf: &mut String<S>) -> Result<(), AppError>
    where
        S: ArrayLength<u8>,
    {
        Command::Iset {
            ch: Channel::Ch1,
            val: self.ch1.iset,
        }
        .append_to_str(cmdbuf)?;

        Command::Iset {
            ch: Channel::Ch2,
            val: self.ch2.iset,
        }
        .append_to_str(cmdbuf)?;

        Ok(())
    }

    #[inline]
    fn vset_cmds<S>(&self, cmdbuf: &mut String<S>) -> Result<(), AppError>
    where
        S: ArrayLength<u8>,
    {
        Command::Vset {
            ch: Channel::Ch1,
            val: self.ch1.vset,
        }
        .append_to_str(cmdbuf)?;

        Command::Vset {
            ch: Channel::Ch2,
            val: self.ch2.vset,
        }
        .append_to_str(cmdbuf)?;
        Ok(())
    }
}

/// What changes when we turn rotary encoder
pub enum ChSelected {
    Both,
    Ch1,
    Ch2,
}
impl ChSelected {
    pub fn next(&self) -> Self {
        match self {
            ChSelected::Both => ChSelected::Ch1,
            ChSelected::Ch1 => ChSelected::Ch2,
            ChSelected::Ch2 => ChSelected::Both,
        }
    }
}

/// What's being modified
pub enum VarSelected {
    V,
    I,
}

impl VarSelected {
    pub fn next(&self) -> Self {
        match self {
            VarSelected::V => VarSelected::I,
            VarSelected::I => VarSelected::V,
        }
    }
}

// Regular info screen, show current values
pub struct InfoScreen {
    pub selected: ChSelected,
    pub ch1: PSChannel,
    pub ch2: PSChannel,
    pub uich: Option<UIChannels>,
    pub vsel: VarSelected,
    pub chsel: ChSelected,
}

impl InfoScreen {
    #[inline]
    pub fn new() -> Self {
        InfoScreen {
            selected: ChSelected::Both,
            ch1: PSChannel::new(),
            ch2: PSChannel::new(),
            uich: None,
            vsel: VarSelected::V,
            chsel: ChSelected::Both,
        }
    }

    /// Handle "on/off" button (try to flip both channels at about the same time)
    #[inline]
    pub fn handle_on_off_button<S>(&mut self, cmdbuf: &mut String<S>) -> Result<(), AppError>
    where
        S: ArrayLength<u8>,
    {
        match self.has_output() {
            Some(ha) => {
                if ha {
                    (Command::Out {
                        ch: Channel::Ch1,
                        on: false,
                    })
                    .append_to_str(cmdbuf)?;

                    (Command::Out {
                        ch: Channel::Ch2,
                        on: false,
                    })
                    .append_to_str(cmdbuf)?;
                } else {
                    (Command::Out {
                        ch: Channel::Ch1,
                        on: true,
                    })
                    .append_to_str(cmdbuf)?;

                    (Command::Out {
                        ch: Channel::Ch2,
                        on: true,
                    })
                    .append_to_str(cmdbuf)?;
                }

                // clear out, wait for next poll
                self.ch1.out = None;
                self.ch2.out = None;
            }
            None => (),
        }

        Ok(())
    }

    pub fn handle_rotary_encoder<S>(
        &mut self,
        re_press_duration: Option<MilliSeconds>,
        re_pressed: bool,
        re_diff: i16,
        cmdbuf: &mut String<S>,
    ) -> Result<(), AppError>
    where
        S: ArrayLength<u8>,
    {
        let now = Instant::now();
        if re_diff != 0 {
            let mut uich: Option<UIChannels> = self.uich.take().or(self.mk_ui_channels());

            uich.as_mut()
                .map(|ch| {
                    let mul = if re_pressed { 1f32 } else { 0.1f32 };
                    let diff = mul * (re_diff as f32);
                    ch.last_change = now;

                    match self.vsel {
                        VarSelected::V => {
                            match self.chsel {
                                ChSelected::Both => {
                                    ch.ch1.vset += diff;
                                    ch.ch2.vset += diff;
                                }
                                ChSelected::Ch1 => {
                                    ch.ch1.vset += diff;
                                }
                                ChSelected::Ch2 => {
                                    ch.ch2.vset += diff;
                                }
                            }

                            ch.fix_range();
                            cmdbuf.clear(); // replace previous command
                            ch.vset_cmds(cmdbuf)
                        }
                        VarSelected::I => {
                            match self.chsel {
                                ChSelected::Both => {
                                    ch.ch1.iset += diff;
                                    ch.ch2.iset += diff;
                                }
                                ChSelected::Ch1 => {
                                    ch.ch1.iset += diff;
                                }
                                ChSelected::Ch2 => {
                                    ch.ch2.iset += diff;
                                }
                            }

                            ch.fix_range();
                            cmdbuf.clear(); // replace previous command
                            ch.iset_cmds(cmdbuf)
                        }
                    }
                })
                .unwrap_or(Ok(()))?;
            self.uich = uich;
        } else {
            match re_press_duration {
                Some(rpd) => {
                    if rpd > MilliSeconds(200) {
                        self.vsel = self.vsel.next();
                    } else {
                        self.chsel = self.chsel.next();
                    }
                }
                None => (),
            }
        }

        Ok(())
    }

    fn mk_ui_channels(&self) -> Option<UIChannels> {
        let now = Instant::now();
        (self.ch1.vset.as_ref().zip(self.ch1.iset.as_ref()))
            .zip(self.ch2.vset.as_ref().zip(self.ch2.iset.as_ref()))
            .map(|((vset1, iset1), (vset2, iset2))| UIChannels {
                ch1: UIChannel::new(*vset1, *iset1),
                ch2: UIChannel::new(*vset2, *iset2),
                last_change: now,
            })
    }

    #[inline]
    pub fn set_query_result<S>(&mut self, q: &Query, s: &String<S>) -> Result<(), AppError>
    where
        S: ArrayLength<u8>,
    {
        let now = Instant::now();

        match self.uich.take() {
            Some(ch) => {
                // note: duration_since blows up after overflow
                if now > ch.last_change
                    && (now.duration_since(ch.last_change).as_cycles() < 3 * SYS_FREQ.0)
                {
                    self.uich = Some(ch); // keep it
                } else {
                    self.uich = None; // timed out, reset from query values
                }
            }
            None => {}
        }

        match q.channel {
            Channel::Ch1 => self.ch1.set_query_result(q, s),
            Channel::Ch2 => self.ch2.set_query_result(q, s),
        }
    }

    #[inline]
    pub fn has_output(&self) -> Option<bool> {
        self.ch1.out.zip(self.ch2.out).map(|(e1, e2)| e1 || e2)
    }
}

/// List SD card root dir, load file
pub struct ProjectFiles {
    pub fnames: Vec<String<U32>, U64>,
    pub selected: usize,
}

impl ProjectFiles {
    pub fn new(sdc: &mut SDCard) -> Result<Self, AppError> {
        let mut fnames = Vec::new();
        sdc.list_projects_files(&mut fnames)?;

        if fnames.is_empty() {
            Err(AppError::ProjectFileError)
        } else {
            Ok(ProjectFiles {
                fnames,
                selected: 0,
            })
        }
    }

    pub fn handle_rotary_encoder(
        &mut self,
        re_press_duration: Option<MilliSeconds>,
        re_diff: i16,
    ) -> Result<Option<String<U32>>, AppError> {
        self.selected = ((self.selected as i16 + re_diff).max(0) as usize).min(self.fnames.len());
        Ok(re_press_duration
            .filter(|pd| pd > &MilliSeconds(100))
            .map(|_| self.fnames[self.selected].clone()))
    }
}

/// UI states
pub enum UI {
    UILoading(&'static str),
    USSBSerial,
    InfoScreen(InfoScreen),
    ProjectFiles(ProjectFiles),
}

/// State of the power supply controller
pub struct PS {
    pub error: Option<AppError>,
    pub ui: UI,
}

impl PS {
    #[inline]
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
