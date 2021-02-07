use cortex_m_semihosting::*;

use crate::types::*;

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
            hprintln!("{:?}!", sdres).unwrap();
            sdres = self.controller.device().init();
        }

        match sdres {
            Ok(_) => {
                hprintln!("SD init OK!").unwrap();
                match self.controller.device().card_size_bytes() {
                    Ok(size) => hprintln!("Card size {}", size).unwrap(),
                    Err(e) => hprintln!("Err: {:?}", e).unwrap(),
                }
                match self.controller.get_volume(embedded_sdmmc::VolumeIdx(0)) {
                    Ok(mut v) => {
                        hprintln!("Volume 0 {:?}", v).unwrap();
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

                        hprintln!("boot buf: {:?}", buf).unwrap();

                        //for c in &buf[0..64] {
                        //    uart_serial.write(*c).map_or((), |_| ())
                        //}
                        //uart_serial.flush().map_or((), |_| ());
                    }
                    Err(e) => hprintln!("Err: {:?}", e).unwrap(),
                }
            }
            Err(e) => hprintln!("{:?}!", e).unwrap(),
        }
    }
}
