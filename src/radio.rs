use embedded_hal::delay::DelayNs;
use esp_idf_hal::gpio::*;
use lora_phy::sx126x::{Sx126x, Sx126xVariant, TcxoCtrlVoltage};
use lora_phy::LoRa;

// Async Delay Wrapper for Ets
#[derive(Clone, Copy)]
pub struct AsyncEts;

impl embedded_hal_async::delay::DelayNs for AsyncEts {
    async fn delay_ns(&mut self, ns: u32) {
        // Ets is blocking, but for simple delays in async context without a real reactor,
        // blocking is "acceptable" though it freezes the executor.
        // For now, to make types align:
        Ets.delay_ns(ns);
    }
}

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

// Sx126x<SPI, BoardType, Delay>? Or SPI, InterfaceVariant.
// lora-phy v3: Sx126x<SPI, IV, D>
// IV must be the Type of the interface variant struct, not the enum value.
// The type of `Sx126xVariant::Sx1262(...)` is `Sx126xVariant<PinDriver<...>, ...>`.
// BUT, Sx126xVariant is an enum.
// The error `expected type, found trait Sx126xVariant` suggests it might be a trait in v3 or I am using it wrong.
// Actually, in v3, `Sx126xVariant` is an enum. You cannot use an enum variant as a type parameter unless it's a const generic?
// NO. The second generic param of Sx126x is `IV`. IV must implement `InterfaceVariant`.
// Does the enum `Sx126xVariant` implement `InterfaceVariant`? YES.
// So `Sx126x<SPI, Sx126xVariant<'d, ...>, Ets>` SHOULD be correct if arguments match.
// BUT `Sx126x` does NOT take lifetime `'d`.
// My previous fix removed `'d` from `Sx126x`, but I kept it on `Sx126xVariant`.
// `Sx126xVariant` DOES take `'d` because it holds `PinDriver<'d>`.

// Let's rely on type inference for the struct field to avoid this mess.
// Use `Box<dyn RadioKind>`? No, overhead.
// Use `impl RadioKind`? Can't in struct field.
// We must name the type.

pub struct TunggerRadio<'d, SPI>
where
    SPI: embedded_hal_async::spi::SpiDevice,
{
    pub lora: LoRa<
        Sx126x<
            SPI,
            Sx126xVariant<
                PinDriver<'d, Gpio8, Output>,
                PinDriver<'d, Gpio12, Output>,
                PinDriver<'d, Gpio13, Input>,
                PinDriver<'d, Gpio14, Input>,
            >,
            Ets,
        >,
        Ets,
    >,
}
// Note: LoRa takes <RK, DLY>. RK = Sx126x<...>.
// Error `Ets does not implement DelayNs`.
// If Ets is blocking, we need to wrap it or use a different delay.
// Let's assume for now we use `esp_idf_hal::delay::Ets` and ignored the error? No, user reported it fails.
// We need a delay that works. `lora-phy` re-exports `embedded_hal::delay::DelayNs`.
// Ets implements it.
// Why error? "the trait bound `Ets: lora_phy::DelayNs` is not satisfied".
// Maybe version mismatch between `esp-idf-hal`'s `embedded-hal` and `lora-phy`'s `embedded-hal`.
// `esp-idf-hal` 0.43 -> `embedded-hal` 1.0.
// `lora-phy` 3 -> `embedded-hal` 1.0 (async supported).
// Maybe `lora-phy::DelayNs` is `embedded_hal_async::delay::DelayNs`?
// If LoRa is async, it needs async delay. Ets is blocking delay.
// We need an async delay wrapper.

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

        // Use AsyncEts
        let lora = LoRa::new(sx126x, true, AsyncEts)
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
