#![doc = r#"
Link: <https://www.buydisplay.com/download/ic/SSD1683.pdf>

The actual controller device that drives the display.

This is flushed commands from the Sram23k256.

"#]

use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiDevice,
};

use crate::display::drivers::{CmdResult, Error};

pub const WIDTH: u16 = 400;
pub const HEIGHT: u16 = 300;

pub struct Ssd1683<Spi, DataCommand, Reset, Busy, Delay> {
    spi: Spi,
    /// flips whether data or a command is being written
    data_command: DataCommand,
    reset: Reset,
    busy: Busy,
    delay: Delay,
}

impl<Spi, DataCommand, Reset, Busy, Delay> Ssd1683<Spi, DataCommand, Reset, Busy, Delay>
where
    Spi: SpiDevice,
    DataCommand: OutputPin,
    Reset: OutputPin<Error = DataCommand::Error>,
    Busy: InputPin<Error = DataCommand::Error>,
    Delay: DelayNs,
{
    pub fn new(
        spi: Spi,
        data_command: DataCommand,
        reset: Reset,
        busy: Busy,
        delay: Delay,
    ) -> Self {
        Self {
            spi,
            data_command,
            reset,
            busy,
            delay,
        }
    }

    pub fn init(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        todo!()
    }

    pub fn refresh(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        todo!()
    }
}
