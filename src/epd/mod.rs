mod sram23k256;

use core::cell::RefCell;

use defmt::info;
use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::{
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    peripherals::{GPIO5, GPIO6, GPIO16, GPIO17, GPIO18, GPIO21, GPIO22, GPIO23, SPI2},
    spi::{
        Mode as SpiMode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};

use crate::epd::sram23k256::Sram23k256;

pub struct Display {}

impl Display {
    /// Takes in the necessary connectors
    pub fn init(pins: DisplayPins) -> Self {
        let display_busy = Input::new(
            pins.paper_display_busy,
            InputConfig::default().with_pull(Pull::None),
        );

        let spi = RefCell::new(
            Spi::new(
                pins.spi_2,
                SpiConfig::default()
                    .with_frequency(Rate::from_mhz(4))
                    .with_mode(SpiMode::_0),
            )
            .expect("SPI2 config")
            .with_sck(pins.sck)
            .with_mosi(pins.mosi)
            .with_miso(pins.miso),
        );

        let paper_display_data_command = Output::new(
            pins.display_data_command,
            Level::High,
            OutputConfig::default(),
        );

        let paper_display_spi = RefCellDevice::new(
            &spi,
            Output::new(
                pins.display_chip_select,
                Level::High,
                OutputConfig::default(),
            ),
            Delay::new(),
        )
        .expect("epd device");

        let ram_spi = RefCellDevice::new(
            &spi,
            Output::new(pins.ram_chip_select, Level::High, OutputConfig::default()),
            Delay::new(),
        )
        .expect("sram device");

        let mut sram = Sram23k256::new(ram_spi);
        if let Err(e) = sram.set_sequential_mode() {
            defmt::error!("SRAM seq mode failed: {:?}", e);
        } else {
            info!("SRAM seq mode OK");
        }

        todo!()
    }
}

pub struct DisplayPins {
    /// This is A2 used as GPIO.
    pub paper_display_busy: GPIO6<'static>,
    pub ram_chip_select: GPIO5<'static>,
    pub spi_2: SPI2<'static>,
    /// "clock"
    pub sck: GPIO21<'static>,
    /// "master out slave in"
    pub mosi: GPIO22<'static>,
    /// "master in slave out"
    pub miso: GPIO23<'static>,
    pub display_data_command: GPIO17<'static>,
    pub display_chip_select: GPIO16<'static>,
    /// this is the only pin for the display that is on the right hand side of the board
    pub display_reset: GPIO18<'static>,
}
