use cortex_m_semihosting::*;

use crate::types::*;

use crate::*;

pub struct SDCard {
    controller: SDCardController,
}

impl SDCard {
    pub fn new(controller: SDCardController) -> SDCard {
        SDCard { controller }
    }

    pub fn init(&mut self) {
        let mut sdres = self.controller.device().init();
        while sdres.is_err() {
            ifcfg!("sdc_debug", hprintln!("{:?}!", sdres));
            sdres = self.controller.device().init();
        }

        match sdres {
            Ok(_) => {
                ifcfg!("sdc_info", hprintln!("SD init OK!"));

                match self.controller.device().card_size_bytes() {
                    Ok(size) => ifcfg!("sdc_info", hprintln!("Card size {}", size)),
                    Err(e) => ifcfg!("sdc_info", hprintln!("Err: {:?}", e)),
                }
                match self.controller.get_volume(embedded_sdmmc::VolumeIdx(0)) {
                    Ok(mut v) => {
                        ifcfg!("sdc_debug", hprintln!("Volume 0 {:?}", v));

                        let r = self.controller.open_root_dir(&v).unwrap();
                        let mut bootf = self
                            .controller
                            .open_file_in_dir(
                                &mut v,
                                &r,
                                "BOOT",
                                embedded_sdmmc::filesystem::Mode::ReadOnly,
                            )
                            .unwrap();

                        let mut buf: [u8; 64] = [0; 64];
                        self.controller.read(&v, &mut bootf, &mut buf).unwrap();

                        ifcfg!("debug_sdc", hprint!("boot buf: {:?}", buf))

                        //if cfg!($cc) { }
                        //for c in &buf[0..64] {
                        //    uart_serial.write(*c).map_or((), |_| ())
                        //}
                        //uart_serial.flush().map_or((), |_| ());
                    }
                    Err(e) => ifcfg!("sdc_info", hprintln!("Err: {:?}", e)),
                }
            }
            Err(e) => ifcfg!("sdc_info", hprintln!("{:?}!", e)),
        }
    }
}
