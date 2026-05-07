pub mod display420;
pub mod sram23k256;
pub mod ssd1683;

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
