#![doc = r#"
Tri-color (white / black / red) framebuffer for the 4.2" 400×300 panel,
backed by the external 23K256 SRAM.

## Why two planes

The SSD1683 controller doesn't think in `TriColor` — it thinks in **two**
independent monochrome bitmaps, one per on-chip RAM bank:

- **RAM1 (B/W plane)** — bit `1` = white pixel, bit `0` = black pixel.
- **RAM2 (red plane)** — bit `1` = red pixel ON, bit `0` = red pixel OFF.
  Red overrides the B/W underneath; if the red bit is set, the pixel
  shows red regardless of what RAM1 says.

So a `TriColor` value gets encoded as a **pair of bits** (`bw_bit`, `red_bit`),
one bit going into each plane at the same `(x, y)` coordinate.

## Memory layout in the external SRAM

The 23K256 has 32 KiB; one full mono plane for the panel is 15 KiB
(`400 * 300 / 8`). We park them back-to-back:

```text
SRAM offset  0x0000  ───── black plane (15,000 bytes) ─────  0x3A98
SRAM offset  0x3A98  ───── red   plane (15,000 bytes) ─────  0x7530
```

Both planes use the same in-memory pixel order: row-major, 8 horizontal
pixels per byte, MSB = leftmost pixel.

## Why the [`PlaneCache`]

A naive `set_pixel` would: read one byte from SRAM, flip one bit, write one
byte back. That's two SPI round-trips per pixel — drawing a single 400-pixel
horizontal line would issue 1,600+ transactions. Drawing a font glyph would
be glacial.

[`PlaneCache`] keeps the most-recently-touched byte buffered in MCU RAM. As
long as consecutive pixel writes hit the same byte (which they do all the
time — adjacent pixels share a byte every 8 columns, vertical lines share a
byte across many rows, etc.), they coalesce into a single eventual write
when the byte changes or [`Display420Tri::flush_caches`] is called. The
cache is bypassed by [`Display420Tri::clear_to`] and
[`Display420Tri::flush_to_panel`], which use bulk SRAM I/O.

## Wire convention (the inversion-bug story)

This code assumes **no controller-side inversion** — i.e., the SSD1683 is
init'd with `DISPLAY_UPDATE_CONTROL_1 = [0x00, 0x80]` (`RamOption::Normal`
on both planes). Under that convention the panel directly mirrors what we
write: RAM1 bit `1` = white, RAM2 bit `1` = red. The encoding tables in
[`encode_fill`] and [`encode_pixel`] follow this directly with **no software
inversion** required.

If you ever flip the controller back to `RamOption::Inverse` for the red
plane, you'd need to invert RAM2 bytes when streaming — or flip the
`red` column of these encoding tables.
"#]

use defmt::{Format, info};
use embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::{ErrorType, SpiDevice},
};

use crate::display::{
    TriColor,
    drivers::{CmdResult, Error, HEIGHT, Sram23k256, Ssd1683, WIDTH},
};

/// Bytes per row of the framebuffer. Each byte packs 8 horizontal pixels,
/// MSB-leftmost.
const BYTES_PER_ROW: u16 = WIDTH / 8;

/// Size of one full plane in bytes (`400 * 300 / 8 = 15_000`).
const PLANE_LEN: usize = (WIDTH as usize * HEIGHT as usize) / 8;

/// SRAM offset where the black/white plane lives.
const BLACK_BASE: u16 = 0;

/// SRAM offset where the red plane lives (right after the black plane).
const RED_BASE: u16 = PLANE_LEN as u16;

/// How many bytes we ship per SPI burst when bulk-copying SRAM ↔ EPD.
/// Sized to fit comfortably in an MCU-side stack buffer; tune up for
/// throughput, down for stack budget.
const FLUSH_CHUNK: usize = 256;

/// A one-byte write-back cache for a single plane.
///
/// `addr` is the SRAM offset of the currently-cached byte, `byte` is its
/// current value (with any in-flight bit flips already applied), and
/// `dirty` says whether `byte` has been modified relative to SRAM and
/// therefore needs to be written back before we move on to a different
/// address.
#[derive(Default)]
struct PlaneCache {
    addr: Option<u16>,
    byte: u8,
    dirty: bool,
}

/// Tri-color framebuffer + the SRAM that backs it.
///
/// Construct with [`Self::new`] (passing in a configured `Sram23k256`) or
/// [`Self::new_from_spi`]. Both planes start with empty caches and
/// undefined SRAM contents — call [`Self::clear_to`] once before drawing
/// to put both planes in a known state.
pub struct Display420Tri<Spi> {
    sram: Sram23k256<Spi>,
    black: PlaneCache,
    red: PlaneCache,
}

impl<Spi> Display420Tri<Spi> {
    /// Wrap an already-constructed `Sram23k256`. Caller is responsible for
    /// calling `sram.set_sequential_mode()` before any drawing.
    pub fn new(sram: Sram23k256<Spi>) -> Self {
        Self {
            sram,
            black: PlaneCache::default(),
            red: PlaneCache::default(),
        }
    }
}

impl<Spi: SpiDevice> Display420Tri<Spi> {
    /// Construct from a raw `SpiDevice` by building the SRAM driver
    /// internally. Caller still needs to put the SRAM in sequential mode
    /// — see the module docs.
    pub fn new_from_spi(spi: Spi) -> Self
    where
        <Spi as ErrorType>::Error: Format,
    {
        let mut sram = Sram23k256::new(spi);

        if let Err(e) = sram.set_sequential_mode() {
            defmt::error!("SRAM seq mode failed: {:?}", e);
        } else {
            info!("SRAM seq mode OK");
        }
        Self::new(sram)
    }

    /// Fill both planes with the bit pattern that encodes `color`, then
    /// drop any cached bytes.
    ///
    /// Uses bulk SRAM writes (`FLUSH_CHUNK`-sized bursts in sequential mode)
    /// so 30 KB of fill goes out in roughly 120 transactions instead of
    /// 30,000.
    pub fn clear_to(&mut self, color: TriColor) -> Result<(), Spi::Error> {
        // Any in-flight cached byte would be stale immediately after the
        // bulk fill, so flush it first rather than leak a half-update.
        self.flush_caches()?;

        let (black_fill, red_fill) = encode_fill(color);
        Self::fill_plane(&mut self.sram, BLACK_BASE, black_fill)?;
        Self::fill_plane(&mut self.sram, RED_BASE, red_fill)?;

        self.black = PlaneCache::default();
        self.red = PlaneCache::default();
        Ok(())
    }

    /// Pump both planes from external SRAM into the SSD1683's on-chip RAM
    /// (the controller's RAM1 + RAM2), then leave the chip ready for a
    /// `refresh()`.
    ///
    /// We don't issue the refresh ourselves — that's the controller's job
    /// (see `Ssd1683::refresh`). This method just gets the bytes there.
    ///
    /// Both planes share the same RAM window and start coordinates; we
    /// reset the address cursor between them because the RAM2 stream needs
    /// to start at `(0, 0)` after the RAM1 stream advanced past `(WIDTH-1,
    /// HEIGHT-1)`.
    pub fn flush_to_panel<DataCommand, Reset, Busy, Delay>(
        &mut self,
        epd: &mut Ssd1683<Spi, DataCommand, Reset, Busy, Delay>,
    ) -> CmdResult<Spi::Error, DataCommand::Error>
    where
        DataCommand: OutputPin,
        Reset: OutputPin<Error = DataCommand::Error>,
        Busy: InputPin<Error = DataCommand::Error>,
        Delay: DelayNs,
    {
        // Make sure any pending single-byte edits are flushed to SRAM
        // before we read SRAM bulk over to the panel.
        self.flush_caches().map_err(Error::Spi)?;

        epd.set_ram_window(0, 0, WIDTH - 1, HEIGHT - 1)?;

        epd.set_ram_address(0, 0)?;
        epd.start_write_ram1()?;
        Self::stream_plane(&mut self.sram, BLACK_BASE, epd)?;

        epd.set_ram_address(0, 0)?;
        epd.start_write_ram2()?;
        Self::stream_plane(&mut self.sram, RED_BASE, epd)?;

        Ok(())
    }

    /// Bulk-fill one plane with a constant byte value, using
    /// `FLUSH_CHUNK`-sized bursts.
    fn fill_plane(sram: &mut Sram23k256<Spi>, base: u16, fill: u8) -> Result<(), Spi::Error> {
        let chunk = [fill; FLUSH_CHUNK];
        let mut written = 0u16;
        while (written as usize) < PLANE_LEN {
            let remaining = PLANE_LEN - written as usize;
            let n = remaining.min(FLUSH_CHUNK);
            sram.write_bulk(base + written, &chunk[..n])?;
            written += n as u16;
        }
        Ok(())
    }

    /// Read one plane out of SRAM in `FLUSH_CHUNK`-sized bursts and forward
    /// each chunk to the SSD1683 as RAM data. The chip's RAM address
    /// counter auto-advances per the data-entry mode set in `init()`, so
    /// we never re-issue the cursor mid-stream.
    fn stream_plane<DataCommand, Reset, Busy, Delay>(
        sram: &mut Sram23k256<Spi>,
        base: u16,
        epd: &mut Ssd1683<Spi, DataCommand, Reset, Busy, Delay>,
    ) -> CmdResult<Spi::Error, DataCommand::Error>
    where
        DataCommand: OutputPin,
        Reset: OutputPin<Error = DataCommand::Error>,
        Busy: InputPin<Error = DataCommand::Error>,
        Delay: DelayNs,
    {
        let mut buf = [0u8; FLUSH_CHUNK];
        let mut sent = 0u16;
        while (sent as usize) < PLANE_LEN {
            let remaining = PLANE_LEN - sent as usize;
            let n = remaining.min(FLUSH_CHUNK);
            sram.read_bulk(base + sent, &mut buf[..n])
                .map_err(Error::Spi)?;
            epd.write_data(&buf[..n])?;
            sent += n as u16;
        }
        Ok(())
    }

    /// Translate `(x, y)` into a byte offset within a single plane.
    /// `None` when the point is off-panel — callers silently drop those
    /// pixels (consistent with embedded-graphics' clipping convention).
    fn pixel_offset(x: i32, y: i32) -> Option<u16> {
        if x < 0 || y < 0 || x >= WIDTH as i32 || y >= HEIGHT as i32 {
            return None;
        }
        Some(y as u16 * BYTES_PER_ROW + (x as u16 >> 3))
    }

    /// Apply one pixel to both planes, going through each plane's cache.
    fn write_pixel(&mut self, x: i32, y: i32, color: TriColor) -> Result<(), Spi::Error> {
        let Some(off) = Self::pixel_offset(x, y) else {
            return Ok(());
        };
        // MSB = leftmost pixel within the byte, so bit 7 is x % 8 == 0.
        let bit = 7 - (x as u16 & 7) as u8;
        let mask = 1u8 << bit;

        let (black_bit, red_bit) = encode_pixel(color);

        Self::touch(
            &mut self.black,
            &mut self.sram,
            BLACK_BASE + off,
            mask,
            black_bit,
        )?;
        Self::touch(&mut self.red, &mut self.sram, RED_BASE + off, mask, red_bit)?;
        Ok(())
    }

    /// Set or clear `mask` within the cached byte at `addr`, loading the
    /// byte from SRAM first if the cache isn't already pointed there.
    /// Writes back to SRAM only on cache eviction or explicit flush — not
    /// on every pixel update.
    fn touch(
        cache: &mut PlaneCache,
        sram: &mut Sram23k256<Spi>,
        addr: u16,
        mask: u8,
        set_bit: bool,
    ) -> Result<(), Spi::Error> {
        // Cache miss: writeback the old line if it was dirty, then load.
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

    /// Push any dirty cached bytes back to SRAM. Called automatically at the
    /// end of `draw_iter` and at the start of `clear_to`/`flush_to_panel`,
    /// so callers normally never have to invoke this directly.
    fn flush_caches(&mut self) -> Result<(), Spi::Error> {
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

/// `TriColor` → `(black_plane_byte_fill, red_plane_byte_fill)` for bulk fill.
///
/// Same logic as [`encode_pixel`] but at byte granularity: `0xFF` = "all 8
/// bits set in this plane," `0x00` = "all 8 cleared." Used by
/// [`Display420Tri::clear_to`].
fn encode_fill(color: TriColor) -> (u8, u8) {
    match color {
        TriColor::White => (0xFF, 0x00),
        TriColor::Black => (0x00, 0x00),
        TriColor::Red => (0xFF, 0xFF),
    }
}

/// `TriColor` → `(black_plane_bit, red_plane_bit)` for a single pixel.
///
/// Truth table under `RamOption::Normal` on both planes (i.e. no
/// controller-side inversion):
///
/// | Color | RAM1 bit | RAM2 bit | Panel renders |
/// | --- | --- | --- | --- |
/// | White | 1 (white) | 0 (red off) | white |
/// | Black | 0 (black) | 0 (red off) | black |
/// | Red   | 1 (white) | 1 (red on)  | red (red overrides the white below) |
fn encode_pixel(color: TriColor) -> (bool, bool) {
    match color {
        TriColor::White => (true, false),
        TriColor::Black => (false, false),
        TriColor::Red => (true, true),
    }
}

impl<Spi> OriginDimensions for Display420Tri<Spi> {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl<Spi: SpiDevice> DrawTarget for Display420Tri<Spi> {
    type Color = TriColor;
    type Error = Spi::Error;

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
