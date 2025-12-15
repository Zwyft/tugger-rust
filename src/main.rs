use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_hal_bus::spi::MutexDevice;
use esp_idf_hal::delay::Ets;
use esp_idf_hal::prelude::*;
use esp_idf_svc::hal as esp_idf_hal;
use log::*;
use std::sync::Mutex;

mod display;
mod hardware;
mod radio;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Starting Tugger Device...");

    let board = hardware::init()?;

    // Create Shared SPI Bus
    // The SpiDriver is owned by the Mutex.
    let spi_bus = Mutex::new(board.spi_bus);

    // Create Devices
    // Radio SPI Device (ExclusiveDevice wrapping MutexDevice wrapping Bus)
    // Actually, lora-phy Sx126x manages CS internally via the 'nss' pin we pass to it?
    // Review: Sx126x::new(nss, ...)
    // If Sx126x manages CS, we should NOT use ExclusiveDevice for it, just MutexDevice.
    // BUT we need to confirm if Sx126x toggles NSS using the PinDriver we give it.
    // Yes, it takes `PinDriver<..., Output>`.
    // So for Radio, we pass MutexDevice + PinDriver.

    // Display SPI Device
    // epd-waveshare Epd2in9::new(...) takes 'cs'.
    // So it also manages CS internally.
    // So we pass MutexDevice + PinDriver for Display too.

    // So NO ExclusiveDevice needed if drivers take raw CS pins!
    let spi_radio_dev = MutexDevice::new(&spi_bus);
    let spi_display_dev = MutexDevice::new(&spi_bus);

    info!("Initializing Radio...");
    let mut radio = radio::TunggerRadio::new(
        spi_radio_dev,
        board.lora_nss,
        board.lora_rst,
        board.lora_busy,
        board.lora_dio1,
    )?;

    radio.configure(&radio::RadioConfig::default())?;
    info!("Radio Initialized.");

    info!("Initializing Display...");
    let mut display = display::TunggerDisplay::new(
        &mut MutexDevice::new(&spi_bus), // Create new handle for access
        board.display_cs,
        board.display_dc,
        board.display_rst,
        board.display_busy,
    )?;

    display.update(&mut MutexDevice::new(&spi_bus), "Booting...")?;
    info!("Display Initialized.");

    loop {
        info!("Tick");
        // Logic loop scaffold
        std::thread::sleep(std::time::Duration::from_secs(5));

        display.update(&mut MutexDevice::new(&spi_bus), "Tick")?;
    }
}
