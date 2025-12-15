use embedded_hal_bus::spi::MutexDevice;
use esp_idf_hal::prelude::*;
use esp_idf_hal::task::block_on;
use esp_idf_svc::hal as esp_idf_hal;
use log::*;
use std::sync::Mutex;

mod display;
mod hardware;
mod radio;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Starting Tugger Device...");

    let board = hardware::init()?;

    // Create Shared SPI Bus
    let spi_bus = Mutex::new(board.spi_bus);

    // Blocking Device for Display
    let mut display_spi = MutexDevice::new(&spi_bus);

    info!("Initializing Display...");
    let mut display = display::TunggerDisplay::new(
        &mut display_spi,
        board.display_cs,
        board.display_dc,
        board.display_rst,
        board.display_busy,
    )?;

    display.update(&mut display_spi, "Booting...")?;
    info!("Display Initialized.");

    // Async Device for Radio
    // Note: embedded_hal_bus::spi::MutexDevice implements embedded_hal_async::spi::SpiDevice
    // IF the underlying bus is also async capable or assumed so.
    // However, esp-idf-hal SpiDriver is inherently blocking.
    // In many cases, blocking impls satisfy Async traits by just completing immediately (poll_ready -> Ready).
    // If logic fails here, we might need to wrap `MutexDevice` in a custom `AsyncAdapter`.
    // But for now, we rely on `embedded-hal-bus` blanket impls.

    let radio_spi = MutexDevice::new(&spi_bus);

    block_on(async {
        info!("Initializing Radio (Async)...");
        // We pass the MutexDevice. If generics align, this works.
        // If compilation fails saying MutexDevice doesn't impl Async SpiDevice,
        // we will need to create a dummy wrapper.

        let mut radio = radio::TunggerRadio::new(
            radio_spi,
            board.lora_nss,
            board.lora_rst,
            board.lora_busy,
            board.lora_dio1,
        )
        .await?;

        radio.configure(&radio::RadioConfig::default()).await?;
        info!("Radio Initialized.");

        Ok::<(), anyhow::Error>(())
    })?;

    loop {
        // Logic loop
        std::thread::sleep(std::time::Duration::from_secs(5));
        display.update(&mut MutexDevice::new(&spi_bus), "Tick")?;
    }
}
