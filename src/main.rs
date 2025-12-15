use esp_idf_hal::prelude::*;
use esp_idf_svc::hal as esp_idf_hal;
use log::*;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Hello, world!");

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;

    loop {
        info!("Tick");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
