use embedded_graphics::Pixel;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Size};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;

use super::Error;
use super::sram23k256::Sram23k256;
use super::ssd1683::{HEIGHT, Ssd1683, WIDTH};

pub const BUF_LEN: usize = (WIDTH as usize * HEIGHT as usize) / 8;
const BYTES_PER_ROW: u16 = WIDTH / 8;
const FLUSH_CHUNK: usize = 256;

pub struct Display420Mono<'a, SPI: SpiDevice> {
    sram: &'a mut Sram23k256<SPI>,
    cached_addr: Option<u16>,
    cached_byte: u8,
    cached_dirty: bool,
}

impl<'a, SPI: SpiDevice> Display420Mono<'a, SPI> {
    pub fn new(sram: &'a mut Sram23k256<SPI>) -> Self {
        Self {
            sram,
            cached_addr: None,
            cached_byte: 0xFF,
            cached_dirty: false,
        }
    }

    pub fn clear_to(&mut self, color: BinaryColor) -> Result<(), SPI::Error> {
        self.flush_cache()?;
        let fill = match color {
            BinaryColor::Off => 0xFFu8,
            BinaryColor::On => 0x00u8,
        };
        let chunk = [fill; FLUSH_CHUNK];
        let mut written = 0u16;
        while (written as usize) < BUF_LEN {
            let remaining = BUF_LEN - written as usize;
            let n = remaining.min(FLUSH_CHUNK);
            self.sram.write_bulk(written, &chunk[..n])?;
            written += n as u16;
        }
        self.cached_addr = None;
        self.cached_dirty = false;
        Ok(())
    }

    pub fn flush_to_panel<DC, RST, BUSY, D>(
        &mut self,
        epd: &mut Ssd1683<SPI, DC, RST, BUSY, D>,
    ) -> Result<(), Error<SPI::Error, DC::Error>>
    where
        DC: OutputPin,
        RST: OutputPin<Error = DC::Error>,
        BUSY: InputPin<Error = DC::Error>,
        D: DelayNs,
    {
        self.flush_cache().map_err(Error::Spi)?;

        epd.set_ram_window(0, 0, WIDTH - 1, HEIGHT - 1)?;
        epd.set_ram_address(0, 0)?;
        epd.start_write_ram1()?;

        let mut buf = [0u8; FLUSH_CHUNK];
        let mut sent = 0u16;
        while (sent as usize) < BUF_LEN {
            let remaining = BUF_LEN - sent as usize;
            let n = remaining.min(FLUSH_CHUNK);
            self.sram
                .read_bulk(sent, &mut buf[..n])
                .map_err(Error::Spi)?;
            epd.write_data(&buf[..n])?;
            sent += n as u16;
        }
        Ok(())
    }

    fn pixel_addr(x: i32, y: i32) -> Option<u16> {
        if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
            return None;
        }
        Some(y as u16 * BYTES_PER_ROW + (x as u16 >> 3))
    }

    fn write_pixel(&mut self, x: i32, y: i32, color: BinaryColor) -> Result<(), SPI::Error> {
        let Some(addr) = Self::pixel_addr(x, y) else {
            return Ok(());
        };
        let bit = 7 - (x as u16 & 7) as u8;
        let mask = 1u8 << bit;

        if self.cached_addr != Some(addr) {
            self.flush_cache()?;
            self.cached_byte = self.sram.read_byte(addr)?;
            self.cached_addr = Some(addr);
            self.cached_dirty = false;
        }

        let new = match color {
            BinaryColor::Off => self.cached_byte | mask,
            BinaryColor::On => self.cached_byte & !mask,
        };
        if new != self.cached_byte {
            self.cached_byte = new;
            self.cached_dirty = true;
        }
        Ok(())
    }

    fn flush_cache(&mut self) -> Result<(), SPI::Error> {
        if let (Some(addr), true) = (self.cached_addr, self.cached_dirty) {
            self.sram.write_byte(addr, self.cached_byte)?;
            self.cached_dirty = false;
        }
        Ok(())
    }
}

impl<SPI: SpiDevice> OriginDimensions for Display420Mono<'_, SPI> {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl<SPI: SpiDevice> DrawTarget for Display420Mono<'_, SPI> {
    type Color = BinaryColor;
    type Error = SPI::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            self.write_pixel(point.x, point.y, color)?;
        }
        self.flush_cache()
    }
}
