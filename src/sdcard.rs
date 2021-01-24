fn init() {
    /*
    let mut cont = embedded_sdmmc::Controller::new(
        embedded_sdmmc::SdMmcSpi::new(sdmmc_spi, sdmmc_cs),
        time_source,
    );

    write!(uart, "Init SD card...").unwrap();
    match cont.device().init() {
        Ok(_) => {
            write!(uart, "OK!\nCard size...").unwrap();
            match cont.device().card_size_bytes() {
                Ok(size) => writeln!(uart, "{}", size).unwrap(),
                Err(e) => writeln!(uart, "Err: {:?}", e).unwrap(),
            }
            write!(uart, "Volume 0...").unwrap();
            match cont.get_volume(embedded_sdmmc::VolumeIdx(0)) {
                Ok(v) => writeln!(uart, "{:?}", v).unwrap(),
                Err(e) => writeln!(uart, "Err: {:?}", e).unwrap(),
            }
        }
        Err(e) => writeln!(uart, "{:?}!", e).unwrap(),
    }
    */
}
