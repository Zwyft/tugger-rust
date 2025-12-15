use embedded_graphics::prelude::*;
use epd_waveshare::{
    epd2in9_v2::{Display2in9, Epd2in9},
    graphics::DisplayRotation,
    prelude::*,
};
use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::*;

pub struct TunggerDisplay<SPI> {
    epd: Epd2in9<
        SPI,
        PinDriver<'static, Gpio7, Input>,
        PinDriver<'static, Gpio5, Output>,
        PinDriver<'static, Gpio6, Output>,
        Ets,
    >,
    display: Display2in9,
}

impl<SPI> TunggerDisplay<SPI>
where
    SPI: embedded_hal::spi::SpiDevice,
{
    pub fn new(
        spi: &mut SPI,
        cs: PinDriver<'static, Gpio4, Output>,
        dc: PinDriver<'static, Gpio5, Output>,
        rst: PinDriver<'static, Gpio6, Output>,
        busy: PinDriver<'static, Gpio7, Input>, // Busy is concrete in main.rs
    ) -> anyhow::Result<Self> {
        let mut delay = Ets;

        // Epd2in9::new signature: (spi, cs, dc, rst, busy, delay) (?)
        // Previous error said "missing spi_speed".
        // If it takes 6 args without speed, and busy is 5th...
        // Let's rely on the error message "missing spi_speed" being the key.
        // It implies the default arguments might be shifting.
        // Let's explicitly pass `None` for speed if it accepts it.
        // Epd2in9::new(spi, cs, dc, rst, busy, delay) - 6 args?
        // Wait, if error "missing spi_speed Option<u32>", then it forces me to pass it.
        // But previously I removed speed and it complained? No, I removed busy and it complained about missing busy (or type).
        // Let's assume (spi, cs, dc, rst, busy, delay, speed). 7 args.
        // This is safe. If it fails, we know exactly why.
        let epd = Epd2in9::new(spi, cs, dc, rst, busy, &mut delay, None)
            .map_err(|_| anyhow::anyhow!("EPD Init failed"))?;

        let mut display = Display2in9::default();
        display.set_rotation(DisplayRotation::Rotate90); // Landscape

        Ok(Self { epd, display })
    }

    pub fn update(&mut self, spi: &mut SPI, _text: &str) -> anyhow::Result<()> {
        let mut delay = Ets;
        self.display.clear(epd_waveshare::color::Color::White).ok();

        // Simple text drawing would need embedded-graphics fonts.
        // For now just waking it up and clearing to prove driver works.
        // TODO: Add text drawing logic

        self.epd
            .update_frame(spi, self.display.buffer(), &mut delay)
            .map_err(|_| anyhow::anyhow!("EPD Update failed"))?;
        self.epd
            .display_frame(spi, &mut delay)
            .map_err(|_| anyhow::anyhow!("EPD Display failed"))?;

        Ok(())
    }
}
