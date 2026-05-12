#![doc = r#"
Link: <https://www.buydisplay.com/download/ic/SSD1683.pdf>

The actual controller device that drives the display.

This is flushed commands from the Sram23k256.

"#]

mod commands;
pub use commands::*;

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

    /// pulse the reset pin from high -> low -> high. Wait for BUSY to drop.
    /// The internal logic is held in reset while RST is low, and on the rising edge, it boots.
    ///
    /// The wait busy after the third edge is because the panel takes a moment to come out
    /// of reset and runs internal boot routines during which BUSY is asserted.
    pub fn reset(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.reset.set_high().map_err(Error::Pin)?;
        self.delay.delay_ms(10);
        self.reset.set_low().map_err(Error::Pin)?;
        self.delay.delay_ms(10);
        self.reset.set_high().map_err(Error::Pin)?;
        self.delay.delay_ms(10);
        self.wait_busy()
    }

    fn command(&mut self, cmd: u8) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.data_command.set_low().map_err(Error::Pin)?;
        self.spi.write(&[cmd]).map_err(Error::Spi)?;
        Ok(())
    }
    fn data(&mut self, bytes: &[u8]) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.data_command.set_high().map_err(Error::Pin)?;
        self.spi.write(bytes).map_err(Error::Spi)?;
        Ok(())
    }

    /// You may need to wait busy
    pub fn run_command(
        &mut self,
        command: WriteCommand,
    ) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command(command.command())?;
        self.wait_busy()?;
        Ok(())
    }

    /// When Busy is high, it means the controller is doing shit.
    ///
    /// So we just wait until its set low.
    fn wait_busy(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        const BUSY_POLL_INTERVAL_MS: u32 = 10;
        const BUSY_TIMEOUT_MS: u32 = 10_000;
        let mut waited_ms: u32 = 0;
        while self.busy.is_high().map_err(Error::Pin)? {
            self.delay.delay_ms(BUSY_POLL_INTERVAL_MS);
            waited_ms = waited_ms.saturating_add(BUSY_POLL_INTERVAL_MS);
            if waited_ms >= BUSY_TIMEOUT_MS {
                return Err(Error::BusyTimeout);
            }
        }
        Ok(())
    }

    pub fn init(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.reset()?;
        self.run_command(WriteCommand::SoftwareReset)?;

        todo!()
    }

    pub fn refresh(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        todo!()
    }
}
