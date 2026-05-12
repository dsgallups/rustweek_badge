use embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiDevice,
};

use crate::display::{
    TriColor,
    drivers::{CmdResult, HEIGHT, Sram23k256, Ssd1683, WIDTH},
};

pub struct Display420Tri<S> {
    sram: Sram23k256<S>,
}

/// This is an abstraction from the static ram held on board.
///
/// It wraps the driver for the RAM, so we issue display commands here,
///
/// And then we will flush the contents of the RAM from this wrapper into
/// the actual controller driver, the ssd1683.
impl<Spi> Display420Tri<Spi> {
    pub fn new(sram: Sram23k256<Spi>) -> Self {
        Self { sram }
    }
}
impl<Spi: SpiDevice> Display420Tri<Spi> {
    pub fn new_from_spi(spi: Spi) -> Self {
        let ram = Sram23k256::new(spi);
        Self::new(ram)
    }

    pub fn flush_to_panel<DataCommand, Reset, Busy, Delay>(
        &self,
        epd: &mut Ssd1683<Spi, DataCommand, Reset, Busy, Delay>,
    ) -> CmdResult<Spi::Error, DataCommand::Error>
    where
        DataCommand: OutputPin,
        Reset: OutputPin<Error = DataCommand::Error>,
        Busy: InputPin<Error = DataCommand::Error>,
        Delay: DelayNs,
    {
        todo!()
    }
    pub fn clear_to(&self, color: TriColor) -> Result<(), Spi::Error> {
        todo!()
    }
}

impl<SPI: SpiDevice> OriginDimensions for Display420Tri<SPI> {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl<SPI: SpiDevice> DrawTarget for Display420Tri<SPI> {
    type Color = TriColor;
    type Error = SPI::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        todo!()
        // for Pixel(point, color) in pixels {
        //     self.write_pixel(point.x, point.y, color)?;
        // }
        // self.flush_caches()
    }
}
