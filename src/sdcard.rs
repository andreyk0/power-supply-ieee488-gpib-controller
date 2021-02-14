use core::ops::FnMut;

use cortex_m_semihosting::*;
use embedded_sdmmc::{filesystem::Mode, SdMmcError, Volume, VolumeIdx};

use crate::prelude::*;
use crate::*;

pub struct SDCard {
    controller: SDCardController,
}

impl SDCard {
    pub fn new(controller: SDCardController) -> SDCard {
        SDCard { controller }
    }

    pub fn init(&mut self) -> Result<Volume, AppError> {
        let mut sdres = self.controller.device().init();
        let mut num_tries = 100;
        while num_tries > 0 && sdres.is_err() {
            num_tries -= 1;
            ifcfg!("sdc_debug", hprintln!("{:?}!", sdres));
            sdres = self.controller.device().init();
        }

        sdres?;

        ifcfg!("sdc_info", hprintln!("SD init OK!"));

        let sd_size = self.controller.device().card_size_bytes()?;
        ifcfg!("sdc_info", hprintln!("Card size {}", sd_size));

        let vol = self.controller.get_volume(VolumeIdx(0))?;
        ifcfg!("sdc_debug", hprintln!("Volume 0 {:?}", vol));

        Ok(vol)
    }

    pub fn send_file<F>(&mut self, fname: &str, mut func: F) -> Result<(), AppError>
    where
        F: FnMut(&[u8]) -> Result<(), AppError>,
    {
        let mut vol = self.init()?;
        let r = self.controller.open_root_dir(&vol)?;
        let mut f = self
            .controller
            .open_file_in_dir(&mut vol, &r, fname, Mode::ReadOnly)?;

        let mut buf: [u8; 128] = [0; 128];
        let mut nbytes = self.controller.read(&vol, &mut f, &mut buf)?;
        while nbytes > 0 {
            ifcfg!("sdc_debug", hprint!("sending: {}", nbytes));
            func(&buf[0..nbytes])?;
            nbytes = self.controller.read(&vol, &mut f, &mut buf)?;
        }

        Ok(())
    }
}

impl From<SdMmcError> for AppError {
    fn from(_: SdMmcError) -> Self {
        AppError::SDError
    }
}

impl From<embedded_sdmmc::Error<SdMmcError>> for AppError {
    fn from(_: embedded_sdmmc::Error<SdMmcError>) -> Self {
        AppError::SDError
    }
}
