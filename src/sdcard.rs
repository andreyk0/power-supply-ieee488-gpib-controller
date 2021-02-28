use cortex_m_semihosting::*;

use core::ops::FnMut;

use embedded_sdmmc::{filesystem::Mode, SdMmcError, Volume, VolumeIdx};

use heapless::{consts::*, String, Vec};

use crate::prelude::*;
use crate::*;

pub struct SDCard {
    controller: SDCardController,
}

impl SDCard {
    pub fn new(mut controller: SDCardController) -> Result<SDCard, AppError> {
        let mut sdres = controller.device().init();
        let mut num_tries = 100;
        while num_tries > 0 && sdres.is_err() {
            num_tries -= 1;
            ifcfg!("sdc_debug", hprintln!("{:?}!", sdres));
            sdres = controller.device().init();
        }

        if sdres.is_err() {
            ifcfg!("sdc_info", hprintln!("SD err {:?}", sdres));
        } else {
            ifcfg!("sdc_info", hprintln!("SD init OK!"));
        }

        sdres?;

        ifcfg!(
            "sdc_debug",
            hprintln!("Card size {}", controller.device().card_size_bytes()?)
        );

        Ok(SDCard { controller })
    }

    #[inline]
    fn get_volume(&mut self) -> Result<Volume, AppError> {
        let volres = self.controller.get_volume(VolumeIdx(0));
        if volres.is_err() {
            ifcfg!("sdc_info", hprintln!("Volume 0 {:?}", volres));
        }
        Ok(volres?)
    }

    #[inline]
    pub fn send_boot_file<F>(&mut self, func: F) -> Result<(), AppError>
    where
        F: FnMut(&[u8]) -> Result<(), AppError>,
    {
        self.send_file("BOOT", func)
    }

    pub fn send_file<F>(&mut self, fname: &str, mut func: F) -> Result<(), AppError>
    where
        F: FnMut(&[u8]) -> Result<(), AppError>,
    {
        let mut vol = self.get_volume()?;
        let dir = self.controller.open_root_dir(&vol)?;

        ifcfg!("sdc_info", hprintln!("send_file {}", fname));

        let mut f = self
            .controller
            .open_file_in_dir(&mut vol, &dir, fname, Mode::ReadOnly)?;

        let mut buf: [u8; 128] = [0; 128];
        let mut nbytes = self.controller.read(&mut vol, &mut f, &mut buf)?;
        while nbytes > 0 {
            ifcfg!("sdc_info", hprintln!("sending: {}", nbytes));
            func(&buf[0..nbytes])?;
            nbytes = self.controller.read(&vol, &mut f, &mut buf)?;
        }

        self.controller.close_file(&vol, f)?;
        self.controller.close_dir(&vol, dir);
        Ok(())
    }

    /// List files in the root directory
    pub fn list_projects_files(
        &mut self,
        fnames: &mut Vec<String<U32>, U64>,
    ) -> Result<(), AppError> {
        ifcfg!("sdc_info", hprintln!("list_proj_files"));

        let vol = self.get_volume()?;
        let dir = self.controller.open_root_dir(&vol)?;

        let mut err = None::<AppError>;
        self.controller.iterate_dir(&vol, &dir, |e| {
            ifcfg!("sdc_debug", hprintln!("entry: {:?}", e.name));

            if !(e.attributes.is_volume() || e.attributes.is_directory()) {
                ifcfg!("sdc_debug", hprintln!("adding: {:?}", e.name));
                let bn = e.name.base_name();
                let res = Vec::from_slice(bn)
                    .map_err(|_| AppError::ProjectFileError)
                    .and_then(|fv| String::from_utf8(fv).map_err(|_| AppError::ProjectFileError))
                    .and_then(|fs| fnames.push(fs).map_err(|_| AppError::ProjectFileError));
                err = err.or(res.err());
            }
        })?;

        self.controller.close_dir(&vol, dir);

        core::slice::heapsort(fnames, |a, b| a.as_str() > b.as_str());

        err.map(|e| Err(e)).unwrap_or(Ok(()))
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
