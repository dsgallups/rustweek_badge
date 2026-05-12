#![doc = r#"
23K256 — a 256-kilobit (32 KiB) serial SRAM by Microchip.

Datasheet: <https://ww1.microchip.com/downloads/en/devicedoc/22100f.pdf>
Instruction set: page 7, Table 2-1. Mode register: page 11, Table 2-2.

## Why we have an external SRAM at all

The ESP32-C6 has tiny RAM (a few hundred KB total, much of it claimed by
Wi-Fi/BLE stacks and Embassy's executor). A single full-resolution tri-color
framebuffer for the 400×300 panel is 30 KB — black plane + red plane, each
15 KB. That's a chunky fraction of what's left for application code.

The Adafruit E-Paper Friend includes a 23K256 chip on the breakout so we can
park the framebuffer off-chip. SPI to external SRAM is slower than direct
memory access, but for an e-paper panel that updates every few seconds it's a
non-issue — the bottleneck is the panel's physical refresh, not the SPI bus.

## How the chip talks

Every transaction starts with a one-byte **opcode**, optionally followed by
a 16-bit big-endian **address**, optionally followed by data. The chip
recognizes four opcodes:

| Opcode | Name | Shape |
| --- | --- | --- |
| `0x01` | Write Status (mode) register | `[0x01, mode]` |
| `0x02` | Write memory | `[0x02, addr_hi, addr_lo, data...]` |
| `0x03` | Read memory | `[0x03, addr_hi, addr_lo]` + N read bytes |
| `0x05` | Read status register | `[0x05]` + 1 read byte |

## Modes (and why we use sequential)

The mode register's top two bits control how the address counter behaves
across multi-byte transfers:

- **Byte mode (`00`, power-on default)** — one address, one byte, then CS
  must rise. Useless for streaming.
- **Page mode (`10`)** — counter auto-increments but **wraps within a
  32-byte page**. Useful for writing aligned 32-byte chunks; useless for our
  15 KB planes.
- **Sequential mode (`01`)** — counter auto-increments across the **full
  32 KiB** before wrapping. This is what we want: one opcode + address,
  then stream as many bytes as we like.

The mode register is volatile — it resets to byte mode on every power-up. So
[`Sram23k256::set_sequential_mode`] has to be called once during init before
any bulk I/O.

## Address space

The chip is 32 KiB. Addresses are 15-bit (0x0000..=0x7FFF) but transmitted as
16 bits (the top bit is "don't care"). Our framebuffer layout:

```text
0x0000 ─┬─────────────────── start of black plane (RAM1 mirror)
        │   15,000 bytes
0x3A98 ─┼─────────────────── start of red plane   (RAM2 mirror)
        │   15,000 bytes
0x7530 ─┴─────────────────── unused tail (~2.5 KB)
```

(Layout constants live in `display/drivers/tricolor.rs`, not here — this
driver is plane-agnostic.)
"#]

use embedded_hal::spi::{Operation, SpiDevice};

/// `0x02` — Write to memory. Followed by 16-bit address (big-endian) then data.
const WRITE: u8 = 0b0000_0010;

/// `0x03` — Read from memory. Followed by 16-bit address (big-endian);
/// subsequent SPI clocks shift bytes back on MISO.
const READ: u8 = 0b0000_0011;

/// `0x05` — Read the mode register. Unused right now; we never need to read
/// the mode back.
#[allow(dead_code)]
const READ_STATUS: u8 = 0b0000_0101;

/// `0x01` — Write the mode register. Followed by a 1-byte mode value.
const WRITE_STATUS: u8 = 0b0000_0001;

// Mode-register top-two-bit values. We only use `SEQUENTIAL_MODE`.
// const BYTE_MODE: u8 = 0;
// const PAGE_MODE: u8 = 0b1000_0000;
/// Mode register value `0b0100_0000` — sequential mode. The address counter
/// auto-increments across the full 32 KiB on bulk reads/writes.
const SEQUENTIAL_MODE: u8 = 0b0100_0000;

/// Driver for the Microchip 23K256 SPI SRAM (32 KiB).
///
/// Owns its [`SpiDevice`] (which already bundles the chip-select line, so this
/// driver never touches CS directly). Construct via [`Self::new`], then call
/// [`Self::set_sequential_mode`] once before any bulk I/O.
pub struct Sram23k256<Spi> {
    spi: Spi,
}

impl<Spi: SpiDevice> Sram23k256<Spi> {
    pub fn new(spi: Spi) -> Self {
        Self { spi }
    }

    /// Put the chip into sequential mode by writing `0x40` to the mode
    /// register (opcode `0x01`).
    ///
    /// Required exactly once at startup before any of the bulk I/O methods.
    /// The mode register is volatile — every power cycle resets it to byte
    /// mode, in which `write_bulk`/`read_bulk` would only transfer one byte
    /// before the address counter stops advancing.
    ///
    /// Note: we don't expose page-mode or byte-mode toggles. If we ever want
    /// to publish this driver standalone, we'd add `set_page_mode` /
    /// `set_byte_mode` and a `mode()` reader using opcode `0x05`.
    pub fn set_sequential_mode(&mut self) -> Result<(), Spi::Error> {
        self.spi.write(&[WRITE_STATUS, SEQUENTIAL_MODE])
    }

    /// Read a single byte from `addr` (opcode `0x03`).
    ///
    /// Used by the framebuffer cache to load the existing byte at a pixel's
    /// address before flipping one bit. For multi-byte transfers, prefer
    /// [`Self::read_bulk`] — it issues the opcode + address once and lets
    /// the chip stream consecutive bytes back.
    pub fn read_byte(&mut self, addr: u16) -> Result<u8, Spi::Error> {
        let header = [READ, (addr >> 8) as u8, addr as u8];
        let mut out = [0u8; 1];
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Read(&mut out)])?;
        Ok(out[0])
    }

    /// Write a single byte to `addr` (opcode `0x02`).
    ///
    /// The command and address share one 4-byte SPI write so CS stays low
    /// for the whole transaction (the chip latches `data` on the trailing
    /// CS rise). Used by the framebuffer cache when flushing a dirty byte.
    pub fn write_byte(&mut self, addr: u16, val: u8) -> Result<(), Spi::Error> {
        self.spi
            .write(&[WRITE, (addr >> 8) as u8, addr as u8, val])
    }

    /// Stream `data.len()` bytes into the chip starting at `addr`
    /// (opcode `0x02` followed by data).
    ///
    /// In sequential mode the address counter auto-increments after each
    /// byte, so a single SPI transaction can rewrite an arbitrarily long
    /// region — useful for `clear_to` (which fills 15 KB per plane).
    ///
    /// **Precondition:** [`Self::set_sequential_mode`] has been called.
    pub fn write_bulk(&mut self, addr: u16, data: &[u8]) -> Result<(), Spi::Error> {
        let header = [WRITE, (addr >> 8) as u8, addr as u8];
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Write(data)])
    }

    /// Stream `out.len()` bytes back from the chip starting at `addr`
    /// (opcode `0x03` followed by reads).
    ///
    /// Used by `flush_to_panel` to pump the framebuffer from external SRAM
    /// into the SSD1683's on-chip RAM via the MCU as a relay. Same
    /// sequential-mode precondition as [`Self::write_bulk`].
    pub fn read_bulk(&mut self, addr: u16, out: &mut [u8]) -> Result<(), Spi::Error> {
        let header = [READ, (addr >> 8) as u8, addr as u8];
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Read(out)])
    }
}
