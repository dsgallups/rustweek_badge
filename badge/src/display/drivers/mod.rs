mod sram23k256;
pub use sram23k256::*;

mod tricolor;
pub use tricolor::*;

mod ssd1683;
pub use ssd1683::*;

#[derive(Debug, defmt::Format)]
pub enum DriverError<Spi, Pin> {
    Spi(Spi),
    Pin(Pin),
    BusyTimeout,
}

impl<Spi, Pin> From<core::convert::Infallible> for DriverError<Spi, Pin> {
    fn from(_: core::convert::Infallible) -> Self {
        unreachable!()
    }
}

pub type CmdResult<SpiErr, DataCommandErr> = Result<(), DriverError<SpiErr, DataCommandErr>>;
