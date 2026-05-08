mod drivers;

#[allow(clippy::module_inception)]
mod display;

use core::cell::RefCell;

use defmt::info;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    peripherals::{GPIO5, GPIO6, GPIO16, GPIO17, GPIO18, GPIO21, GPIO22, GPIO23, SPI2},
    spi::{
        Mode as SpiMode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};

pub static DRAW_CHANNEL: Channel<CriticalSectionRawMutex, DrawCommand, 4> = Channel::new();

use drivers::sram23k256::Sram23k256;
use shared::{DrawCommand, LightCommand};

use crate::display::{
    display::Display,
    drivers::{display420tri::Display420Tri, ssd1683::Ssd1683},
};

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

#[embassy_executor::task]
pub async fn run_display(pins: DisplayPins) {
    let paper_display_busy = Input::new(
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

    // this one is on the right hand side
    let paper_display_reset = Output::new(pins.display_reset, Level::High, OutputConfig::default());

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

    let display = Display420Tri::new(sram);

    let display_controller = Ssd1683::new(
        paper_display_spi,
        paper_display_data_command,
        paper_display_reset,
        paper_display_busy,
        Delay::new(),
    );

    let mut device = Display::new(display, display_controller);

    loop {
        display_loop(&mut device).await;
    }
}

async fn display_loop<'other_io, 'spi>(display: &mut Display<'other_io, 'spi>) {
    let command = DRAW_CHANNEL.receive().await;

    match command {
        DrawCommand::Line => {
            todo!()
        }
        DrawCommand::Clear => {
            display
                .display()
                .clear_to(drivers::display420tri::TriColor::Black)
                .unwrap();
        }
        DrawCommand::Flush => {
            //todo!()
        }
    }

    todo!()
}
