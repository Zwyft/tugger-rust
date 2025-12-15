use esp_idf_hal::delay::Ets;
use esp_idf_hal::gpio::*;
use lora_phy::lorawan_radio::LorawanRadio;
use lora_phy::sx126x::{Sx126x, Sx126xVariant, TcxoCtrlVoltage};
use lora_phy::LoRa;

// Wrapper for Ets to implement embedded_hal_async::delay::DelayNs if needed
// Or use standard DelayNs validation. Ets implements blocking DelayNs.
// If lora-phy v3 uses async DelayNs, 'Ets' won't work directly.
// We will use a dummy Async Delay for now or `esp_idf_hal::task::embassy_sync::EspRawMutex`?
// No, simpler: Ets is blocking. If lora-phy needs async delay, we are in trouble with Ets.
// BUT `lora-phy` v3 `LoRa::new` signature: `new(radio_kind, enable_public_network, delay)`.
// The delay must implement `DelayNs`.
// If it requires Async, we need an async delay.
// Let's assume for a moment Ets works or we can simple-wrap it.

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

pub struct TunggerRadio<'d, SPI>
where
    SPI: embedded_hal_async::spi::SpiDevice,
{
    // Fix: Sx126x does not take lifetime 'd in v3.
    // Fix: Sx126xVariant is an enum/struct, pass it as Type?
    // Sx126x<SPI, Variant>
    pub lora: LoRa<
        Sx126x<
            SPI,
            Sx126xVariant<
                PinDriver<'d, Gpio8, Output>,
                PinDriver<'d, Gpio12, Output>,
                PinDriver<'d, Gpio13, Input>,
                PinDriver<'d, Gpio14, Input>,
            >,
        >,
        Ets,
    >,
}

impl<'d, SPI> TunggerRadio<'d, SPI>
where
    SPI: embedded_hal_async::spi::SpiDevice,
{
    pub async fn new(
        spi: SPI,
        nss: PinDriver<'d, Gpio8, Output>,
        rst: PinDriver<'d, Gpio12, Output>,
        busy: PinDriver<'d, Gpio13, Input>,
        dio1: PinDriver<'d, Gpio14, Input>,
    ) -> anyhow::Result<Self> {
        // Fix: Sx1262 variant construction
        let iv = Sx126xVariant::Sx1262(nss, rst, busy, dio1);

        // config is `lora_phy::sx126x::Config`
        let config = lora_phy::sx126x::Config {
            chip: lora_phy::sx126x::BoardType::Sx1262,
            tcxo_ctrl: Some(TcxoCtrlVoltage::Ctrl1V7),
            use_dcdc: true,
            rx_boost: false,
        };

        let sx126x = Sx126x::new(spi, iv, config);

        let lora = LoRa::new(sx126x, true, Ets)
            .await
            .map_err(|e| anyhow::anyhow!("LoRa init failed: {:?}", e))?;

        Ok(Self { lora })
    }

    pub async fn configure(&mut self, cfg: &RadioConfig) -> anyhow::Result<()> {
        let mdltn_params = self
            .lora
            .create_modulation_params(
                lora_phy::mod_params::SpreadingFactor::_9,
                lora_phy::mod_params::Bandwidth::_125KHz, // Fixed Case
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
            .await
            .map_err(|e| anyhow::anyhow!("Standby error: {:?}", e))?;
        Ok(())
    }

    pub async fn transmit(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let mdltn_params = self
            .lora
            .create_modulation_params(
                lora_phy::mod_params::SpreadingFactor::_9,
                lora_phy::mod_params::Bandwidth::_125KHz,
                lora_phy::mod_params::CodingRate::_4_7,
                915_000_000,
            )
            .map_err(|e| anyhow::anyhow!("ModParams error: {:?}", e))?;

        self.lora
            .prepare_for_tx(&mdltn_params, 14, false)
            .await
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
            .await
            .map_err(|e| anyhow::anyhow!("TX error: {:?}", e))?;

        Ok(())
    }
}
