// embedded-hal-bus 0.1 location:
use embedded_hal_bus::spi::MutexDevice;
use esp_idf_hal::gpio::*;
use esp_idf_hal::task::block_on;
use esp_idf_svc::hal as esp_idf_hal;
use log::*;
use std::sync::Mutex; // Import traits for downgrade

mod display;
mod hardware;
mod radio;

// Wrapper to allow blocking SPI to be used where Async SPI is expected
use embedded_hal_async::spi::SpiDevice;

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

    // Downgrade busy pin for display to generic Input
    // board.display_busy is likely Gpio7
    let display_busy = board.display_busy; // It's a concrete PinDriver

    info!("Initializing Display...");
    let mut display = display::TunggerDisplay::new(
        &mut display_spi,
        board.display_cs,
        board.display_dc,
        board.display_rst,
        display_busy,
    )?;

    display.update(&mut display_spi, "Booting...")?;
    info!("Display Initialized.");

    // Async Device for Radio
    // Wrap the blocking MutexDevice in our adapter
    let radio_spi = BlockingAsyncSpi(MutexDevice::new(&spi_bus));

    // Downgrade pins for Radio
    let lora_nss = board.lora_nss.into_any_output();
    let lora_rst = board.lora_rst.into_any_output();
    let lora_busy = board.lora_busy.into_any_input();
    let lora_dio1 = board.lora_dio1.into_any_input();

    block_on(async {
        info!("Initializing Radio (Async)...");

        // Initialize TimerDriver for LoRa delay
        let timer_service = esp_idf_hal::timer::TimerService::new()?;
        let timer = timer_service.timer00;
        let delay_driver =
            esp_idf_hal::timer::TimerDriver::new(timer, &esp_idf_hal::timer::TimerConfig::new())?;

        let mut radio = radio::TunggerRadio::new(
            radio_spi,
            lora_nss,
            lora_rst,
            lora_busy,
            lora_dio1,
            delay_driver,
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
