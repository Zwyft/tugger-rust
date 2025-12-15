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
        busy: PinDriver<'static, Gpio7, Input>,
    ) -> anyhow::Result<Self> {
        let mut delay = Ets;

        // Epd2in9::new signature: (spi, cs, busy, dc, rst, delay)
        let epd = Epd2in9::new(spi, cs, busy, dc, rst, &mut delay)
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
