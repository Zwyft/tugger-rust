// embedded-hal-bus 0.1 location:
use embedded_hal_bus::spi::MutexDevice;
use esp_idf_hal::task::block_on;
use esp_idf_svc::hal as esp_idf_hal;
use log::*;
use std::sync::Mutex;

mod display;
mod hardware;
mod radio;

// Wrapper to allow blocking SPI to be used where Async SPI is expected
// This blocks the executor thread, which is acceptable in esp-idf threaded context.
pub struct BlockingAsyncSpi<T>(T);

impl<T: embedded_hal::spi::ErrorType> embedded_hal::spi::ErrorType for BlockingAsyncSpi<T> {
    type Error = T::Error;
}

impl<T: embedded_hal::spi::SpiDevice> embedded_hal_async::spi::SpiDevice for BlockingAsyncSpi<T> {
    async fn transaction(
        &mut self,
        operations: &mut [embedded_hal::spi::Operation<'_, u8>],
    ) -> Result<(), T::Error> {
        self.0.transaction(operations)
    }
}

fn main() -> anyhow::Result<()> {
    // Check-cfg are handled in build.rs
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
    // Wrap the blocking MutexDevice in our adapter
    let radio_spi = BlockingAsyncSpi(MutexDevice::new(&spi_bus));

    // Display is moved into the block
    block_on(async {
        info!("Initializing Radio (Async)...");

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

        loop {
            // Logic loop
            std::thread::sleep(std::time::Duration::from_secs(5));

            display.update(&mut display_spi, "Tick")?;
            info!("Tick");
        }
    })?;

    Ok(())
}
