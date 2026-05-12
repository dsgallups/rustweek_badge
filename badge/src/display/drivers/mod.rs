mod sram23k256;
pub use sram23k256::*;

mod tricolor;
pub use tricolor::*;

mod ssd1683;
pub use ssd1683::*;

#[derive(Debug, defmt::Format)]
pub enum Error<Spi, Pin> {
    Spi(Spi),
    Pin(Pin),
    BusyTimeout,
}

impl<Spi, Pin> From<core::convert::Infallible> for Error<Spi, Pin> {
    fn from(_: core::convert::Infallible) -> Self {
        unreachable!()
    }
}

pub type DriverError<SPI, DC> = Error<<SPI as embedded_hal::spi::ErrorType>::Error, DC>;

pub type CmdResult<SpiErr, DataCommandErr> = Result<(), Error<SpiErr, DataCommandErr>>;
