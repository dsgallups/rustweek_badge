use embedded_graphics::Pixel;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{OriginDimensions, Size};
use embedded_graphics::pixelcolor::PixelColor;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;

use super::Error;
use super::sram23k256::Sram23k256;
use super::ssd1683::{HEIGHT, Ssd1683, WIDTH};

pub const PLANE_LEN: usize = (WIDTH as usize * HEIGHT as usize) / 8;
const BYTES_PER_ROW: u16 = WIDTH / 8;
const FLUSH_CHUNK: usize = 256;

const BLACK_BASE: u16 = 0;
const RED_BASE: u16 = PLANE_LEN as u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum TriColor {
    White,
    Black,
    Red,
}

impl PixelColor for TriColor {
    type Raw = ();
}

#[derive(Default)]
struct PlaneCache {
    addr: Option<u16>,
    byte: u8,
    dirty: bool,
}

pub struct Display420Tri<'a, SPI: SpiDevice> {
    sram: &'a mut Sram23k256<SPI>,
    black: PlaneCache,
    red: PlaneCache,
}

impl<'a, SPI: SpiDevice> Display420Tri<'a, SPI> {
    pub fn new(sram: &'a mut Sram23k256<SPI>) -> Self {
        Self {
            sram,
            black: PlaneCache::default(),
            red: PlaneCache::default(),
        }
    }

    pub fn clear_to(&mut self, color: TriColor) -> Result<(), SPI::Error> {
        self.flush_caches()?;
        let (black_fill, red_fill) = encode_fill(color);
        Self::fill_plane(self.sram, BLACK_BASE, black_fill)?;
        Self::fill_plane(self.sram, RED_BASE, red_fill)?;
        self.black = PlaneCache::default();
        self.red = PlaneCache::default();
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
        self.flush_caches().map_err(Error::Spi)?;

        epd.set_ram_window(0, 0, WIDTH - 1, HEIGHT - 1)?;
        epd.set_ram_address(0, 0)?;
        epd.start_write_ram1()?;
        Self::stream_plane(self.sram, BLACK_BASE, epd)?;

        epd.set_ram_address(0, 0)?;
        epd.start_write_ram2()?;
        Self::stream_plane(self.sram, RED_BASE, epd)?;

        Ok(())
    }

    fn fill_plane(sram: &mut Sram23k256<SPI>, base: u16, fill: u8) -> Result<(), SPI::Error> {
        let chunk = [fill; FLUSH_CHUNK];
        let mut written = 0u16;
        while (written as usize) < PLANE_LEN {
            let n = (PLANE_LEN - written as usize).min(FLUSH_CHUNK);
            sram.write_bulk(base + written, &chunk[..n])?;
            written += n as u16;
        }
        Ok(())
    }

    fn stream_plane<DC, RST, BUSY, D>(
        sram: &mut Sram23k256<SPI>,
        base: u16,
        epd: &mut Ssd1683<SPI, DC, RST, BUSY, D>,
    ) -> Result<(), Error<SPI::Error, DC::Error>>
    where
        DC: OutputPin,
        RST: OutputPin<Error = DC::Error>,
        BUSY: InputPin<Error = DC::Error>,
        D: DelayNs,
    {
        let mut buf = [0u8; FLUSH_CHUNK];
        let mut sent = 0u16;
        while (sent as usize) < PLANE_LEN {
            let n = (PLANE_LEN - sent as usize).min(FLUSH_CHUNK);
            sram.read_bulk(base + sent, &mut buf[..n])
                .map_err(Error::Spi)?;
            epd.write_data(&buf[..n])?;
            sent += n as u16;
        }
        Ok(())
    }

    fn pixel_offset(x: i32, y: i32) -> Option<u16> {
        if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
            return None;
        }
        Some(y as u16 * BYTES_PER_ROW + (x as u16 >> 3))
    }

    fn write_pixel(&mut self, x: i32, y: i32, color: TriColor) -> Result<(), SPI::Error> {
        let Some(off) = Self::pixel_offset(x, y) else {
            return Ok(());
        };
        let bit = 7 - (x as u16 & 7) as u8;
        let mask = 1u8 << bit;

        let (black_bit, red_bit) = encode_pixel(color);

        Self::touch(
            &mut self.black,
            self.sram,
            BLACK_BASE + off,
            mask,
            black_bit,
        )?;
        Self::touch(&mut self.red, self.sram, RED_BASE + off, mask, red_bit)?;
        Ok(())
    }

    fn touch(
        cache: &mut PlaneCache,
        sram: &mut Sram23k256<SPI>,
        addr: u16,
        mask: u8,
        set_bit: bool,
    ) -> Result<(), SPI::Error> {
        if cache.addr != Some(addr) {
            if let (Some(prev), true) = (cache.addr, cache.dirty) {
                sram.write_byte(prev, cache.byte)?;
            }
            cache.byte = sram.read_byte(addr)?;
            cache.addr = Some(addr);
            cache.dirty = false;
        }
        let new = if set_bit {
            cache.byte | mask
        } else {
            cache.byte & !mask
        };
        if new != cache.byte {
            cache.byte = new;
            cache.dirty = true;
        }
        Ok(())
    }

    fn flush_caches(&mut self) -> Result<(), SPI::Error> {
        if let (Some(addr), true) = (self.black.addr, self.black.dirty) {
            self.sram.write_byte(addr, self.black.byte)?;
            self.black.dirty = false;
        }
        if let (Some(addr), true) = (self.red.addr, self.red.dirty) {
            self.sram.write_byte(addr, self.red.byte)?;
            self.red.dirty = false;
        }
        Ok(())
    }
}

fn encode_fill(color: TriColor) -> (u8, u8) {
    match color {
        TriColor::White => (0xFF, 0x00),
        TriColor::Black => (0x00, 0x00),
        TriColor::Red => (0xFF, 0xFF),
    }
}

fn encode_pixel(color: TriColor) -> (bool, bool) {
    match color {
        TriColor::White => (true, false),
        TriColor::Black => (false, false),
        TriColor::Red => (true, true),
    }
}

impl<SPI: SpiDevice> OriginDimensions for Display420Tri<'_, SPI> {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl<SPI: SpiDevice> DrawTarget for Display420Tri<'_, SPI> {
    type Color = TriColor;
    type Error = SPI::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            self.write_pixel(point.x, point.y, color)?;
        }
        self.flush_caches()
    }
}
