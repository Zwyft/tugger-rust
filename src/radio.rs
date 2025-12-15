use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::*;
use lora_phy::lorawan_radio::LorawanRadio;
use lora_phy::sx126x::{Sx126x, Sx126xVariant};
use lora_phy::LoRa;

pub struct RadioConfig {
    pub frequency: u32,
    pub bandwidth: u32,
    pub spreading_factor: u8,
    pub coding_rate: u8,
    pub output_power: i8,
}

impl Default for RadioConfig {
    fn default() -> Self {
        Self {
            frequency: 915_000_000,
            bandwidth: 125_000,
            spreading_factor: 9,
            coding_rate: 7,
            output_power: 14,
        }
    }
}

pub struct TunggerRadio<'d, SPI> {
    pub lora: LoRa<
        Sx126x<
            'd,
            SPI,
            PinDriver<'d, Gpio8, Output>,
            PinDriver<'d, Gpio12, Output>,
            PinDriver<'d, Gpio13, Input>,
            PinDriver<'d, Gpio14, Input>,
        >,
        Ets,
    >,
}

impl<'d, SPI> TunggerRadio<'d, SPI>
where
    SPI: embedded_hal::spi::SpiDevice,
{
    pub fn new(
        spi: SPI,
        nss: PinDriver<'d, Gpio8, Output>,
        rst: PinDriver<'d, Gpio12, Output>,
        busy: PinDriver<'d, Gpio13, Input>,
        dio1: PinDriver<'d, Gpio14, Input>,
    ) -> anyhow::Result<Self> {
        let sx126x = Sx126x::new(nss, rst, busy, spi, Ets);

        let lora = LoRa::new(sx126x, true, Ets)
            .map_err(|e| anyhow::anyhow!("LoRa init failed: {:?}", e))?;

        Ok(Self { lora })
    }

    pub fn configure(&mut self, cfg: &RadioConfig) -> anyhow::Result<()> {
        let mdltn_params = self
            .lora
            .create_modulation_params(
                lora_phy::mod_params::SpreadingFactor::_9,
                lora_phy::mod_params::Bandwidth::_125KHZ,
                lora_phy::mod_params::CodingRate::_4_7,
                cfg.frequency,
            )
            .map_err(|e| anyhow::anyhow!("ModParams error: {:?}", e))?;

        let tx_params = self
            .lora
            .create_tx_packet_params(8, false, true, false, &mdltn_params)
            .map_err(|e| anyhow::anyhow!("TxParams error: {:?}", e))?;

        self.lora
            .enter_standby()
            .map_err(|e| anyhow::anyhow!("Standby error: {:?}", e))?;
        Ok(())
    }

    pub fn transmit(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let mdltn_params = self
            .lora
            .create_modulation_params(
                lora_phy::mod_params::SpreadingFactor::_9,
                lora_phy::mod_params::Bandwidth::_125KHZ,
                lora_phy::mod_params::CodingRate::_4_7,
                915_000_000,
            )
            .map_err(|e| anyhow::anyhow!("ModParams error: {:?}", e))?;

        self.lora
            .prepare_for_tx(&mdltn_params, 14, false)
            .map_err(|e| anyhow::anyhow!("PrepareTx error: {:?}", e))?;

        self.lora
            .tx(
                &mdltn_params,
                &mut self
                    .lora
                    .create_tx_packet_params(8, false, true, false, &mdltn_params)
                    .unwrap(),
                data,
                0xFFFFFF,
            )
            .map_err(|e| anyhow::anyhow!("TX error: {:?}", e))?;

        Ok(())
    }
}
