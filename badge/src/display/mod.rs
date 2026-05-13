pub mod drivers;

mod color;
use alloc::string::ToString;
pub use color::*;

#[allow(clippy::module_inception)]
mod display;

use core::cell::RefCell;

use defmt::{error, info};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};
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

pub static DRAW_CHANNEL: Channel<CriticalSectionRawMutex, DrawCommand, 4> = Channel::new();

use shared::{Color, DrawCommand};

use crate::display::{
    display::Display,
    drivers::{Display420Tri, Ssd1683},
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

    // So refcell devices allow us to communicate with a particular chip
    // over common SPI pins by setting another active pin low.
    //
    // If you set all of these "chip select" pins low, then all receiving peripherals will
    // recieve the data. if you set them all high, none of them will. And of course,
    // If you set one low at a time, that particular device will only receive data over SPI.
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

    let display = Display420Tri::new_from_spi(ram_spi);

    let display_controller = Ssd1683::new(
        paper_display_spi,
        paper_display_data_command,
        paper_display_reset,
        paper_display_busy,
        Delay::new(),
    );

    let mut device = Display::new(display, display_controller);
    // device.init();
    info!("Display initialized");

    if let Err(e) = device.debug() {
        let val = e.0.as_ref();
        info!("Error: {}", val);
    };

    // // Minimal flash self-test — bypasses the SRAM-backed framebuffer and the
    // // tri-color encoding entirely. We fill the chip's own RAM1/RAM2 with
    // // constant bytes and trigger a full refresh, so any failure here is at
    // // the SSD1683 interface (SPI, BUSY, init sequence) and not in the
    // // upstream framebuffer code.
    // //
    // // Patterns:
    // // - (0x00, 0xFF) — RAM1 all 0 (black), RAM2 all 1 (red OFF under the
    // //   panel's inverted red plane) → panel should drive **all-black**.
    // // - (0xFF, 0xFF) — RAM1 all 1 (white), RAM2 all 1 (red OFF) → panel
    // //   should drive **all-white**.
    // // - (0xFF, 0x00) — RAM1 all 1 (white), RAM2 all 0 (red ON) → panel
    // //   should drive **all-red** (red overrides B/W).
    // for (label, bw_byte, red_byte) in [
    //     ("black", 0x00u8, 0xFFu8),
    //     ("white", 0xFFu8, 0xFFu8),
    //     ("red", 0xFFu8, 0x00u8),
    // ] {
    //     info!(
    //         "(DISPLAY) flash-test: filling {} (bw=0x{:02X}, red=0x{:02X})",
    //         label, bw_byte, red_byte
    //     );
    //     match device.controller().flash_test(bw_byte, red_byte) {
    //         Ok(()) => info!("(DISPLAY) flash-test {} refresh completed", label),
    //         Err(e) => error!("(DISPLAY) flash-test {} failed: {:?}", label, e),
    //     }
    //     info!("(DISPLAY) flash-test {} holding 5s", label);
    //     Timer::after(Duration::from_secs(5)).await;
    // }

    // loop {
    //     let command = DRAW_CHANNEL.receive().await;
    //     handle_command(&mut device, command);
    // }
}

fn handle_command(display: &mut Display<'_, '_>, command: DrawCommand) {
    match command {
        DrawCommand::Line { start, end, color } => {
            let line = Line::new(
                Point::new(start.x as i32, start.y as i32),
                Point::new(end.x as i32, end.y as i32),
            );
            if let Err(e) = line
                .into_styled(PrimitiveStyle::with_stroke(tri_from(color), 1))
                .draw(display.display())
            {
                error!("line draw failed: {:?}", e);
            }
            info!("(DISPLAY) Line command performed!");
        }
        DrawCommand::Debug => {
            info!("(DISPLAY) Executing debug!");

            if let Err(e) = display.display().clear_to(TriColor::White) {
                error!("(DISPLAY) debug clear failed: {:?}", e);
                return;
            }

            let diagonal = Line::new(Point::new(0, 0), Point::new(399, 299))
                .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1));
            if let Err(e) = diagonal.draw(display.display()) {
                error!("(DISPLAY) debug diagonal failed: {:?}", e);
            }

            let red_stripe = Line::new(Point::new(0, 150), Point::new(399, 150))
                .into_styled(PrimitiveStyle::with_stroke(TriColor::Red, 3));
            if let Err(e) = red_stripe.draw(display.display()) {
                error!("(DISPLAY) debug stripe failed: {:?}", e);
            }

            display.flush();
            info!("(DISPLAY) Debug pattern drawn + flushed");
        }
        DrawCommand::Clear { color } => {
            if let Err(e) = display.display().clear_to(tri_from(color)) {
                error!("(DISPLAY) clear failed: {:?}", e);
            }
            info!("(DISPLAY) Clear command performed!");
        }
        DrawCommand::Flush => {
            info!("(DISPLAY) Flush command performed!");
            display.flush();
        }
    }
}

fn tri_from(c: Color) -> TriColor {
    match c {
        Color::White => TriColor::White,
        Color::Black => TriColor::Black,
        Color::Red => TriColor::Red,
    }
}
