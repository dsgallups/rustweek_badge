use embedded_hal::spi::SpiDevice;

use crate::display::drivers::Sram23k256;

pub struct Display420Tri<S> {
    sram: Sram23k256<S>,
}

/// This is an abstraction from the static ram held on board.
///
/// It wraps the driver for the RAM, so we issue display commands here,
///
/// And then we will flush the contents of the RAM from this wrapper into
/// the actual controller driver, the ssd1683.
impl<S> Display420Tri<S> {
    pub fn new(sram: Sram23k256<S>) -> Self {
        Self { sram }
    }
}
impl<S: SpiDevice> Display420Tri<S> {
    pub fn new_from_spi(spi: S) -> Self {
        let ram = Sram23k256::new(spi);
        Self::new(ram)
    }
}
