#![doc = r#"
SSD1683 driver — the controller chip inside our 4.2" 400×300 e-paper panel.

Datasheet: <https://www.buydisplay.com/download/ic/SSD1683.pdf>

## What this chip is

The SSD1683 sits between the MCU and the actual e-ink ink particles. The panel
itself is "dumb glass" — a grid of transparent capsules full of black and white
(and red, on tri-color) particles suspended in fluid. The SSD1683 generates the
analog voltage waveforms that physically push those particles into the right
positions.

Our job here is to *configure* the chip, *stream pixel data* into its on-chip
RAM, and *trigger* a refresh. The chip handles the analog physics.

## How we talk to it

The chip is a 4-wire SPI slave plus three side-band pins:

- **CS** (chip select) — pulled low while we're talking to this chip vs. the
  shared SRAM on the same SPI bus. Handled by the `embedded-hal` `SpiDevice`
  wrapper, so the driver never touches it directly.
- **DC** (data/command) — pulled **low** before sending an opcode byte,
  **high** before sending the data bytes that parameterize it. Plain SPI
  can't distinguish command from data on its own, so the chip adds this pin.
- **RST** (reset) — pulled low for ~10ms to force a hardware reset, like a
  power-cycle. We do this once at startup.
- **BUSY** — driven *by the chip*, read *by the MCU*. High means the chip is
  busy doing something physical (running a refresh waveform, loading its OTP
  LUTs, etc.). We have to wait for it to drop before sending the next
  command. The [`Ssd1683::wait_busy`] helper polls it in a loop.

## On-chip RAM banks

The chip has **two framebuffers** (RAM1 and RAM2), each sized to the panel:

- **RAM1** — black/white plane. Bit `1` = white pixel, bit `0` = black pixel.
  Written via opcode `0x24` (`WRITE_RAM_BW`).
- **RAM2** — red plane on tri-color panels (bit `1` = red ON), or the
  previous-frame reference on mono partial refresh. Written via opcode `0x26`
  (`WRITE_RAM_RED`).

The driver doesn't allocate framebuffers locally — those live in the external
SRAM (the 23K256) so we don't burn the MCU's tiny RAM budget on a 30 KB image.
Higher-level code in `display/drivers/tricolor.rs` and friends manages the
SRAM-backed framebuffers and streams them in.

## Lifecycle

```text
new() ───► init() ───► (write RAM) ───► refresh() ───► (sleep() optional)
              ▲                              │
              │                              │
              └────── flush more frames ─────┘
```

[`Ssd1683::init`] does a hardware reset, then walks the documented init
sequence (data-entry mode, no-inversion display-update control, border
waveform, temperature source, RAM window). After it returns, the chip is
idle, RAM contents are undefined, and you're ready to stream pixels.

Each named init step is also a public method on its own, so callers can
re-issue any one of them later without re-doing the full init — useful when
you change the active RAM window between draws.
"#]
// The driver intentionally exposes the full SSD1683 surface (named opcodes,
// alternate enum variants like `RamOption::Inverse`, the sleep/refresh
// lifecycle). Some of it isn't reached from `main.rs` yet because
// `_fordebug.rs` is unwired and `tricolor.rs` is still stubbed.
#![allow(dead_code)]

mod commands;
pub use commands::*;

use defmt::info;
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiDevice,
};

use crate::display::drivers::{CmdResult, DriverError};

/// Width of the 4.2" panel in pixels (horizontal source outputs).
pub const WIDTH: u16 = 400;
/// Height of the 4.2" panel in pixels (vertical gate outputs).
pub const HEIGHT: u16 = 300;

/// Driver for the SSD1683 e-paper controller.
///
/// Generic over its four side-band pins (`DataCommand`, `Reset`, `Busy`) plus
/// the SPI device and a delay source. All three pins share the same error
/// type so we can collapse them into a single [`Error::Pin`] variant.
pub struct Ssd1683<Spi, DataCommand, Reset, Busy, Delay> {
    spi: Spi,
    /// DC pin. Low = the next SPI byte is a command opcode; high = the next
    /// bytes are data parameters for the previous command.
    data_command: DataCommand,
    /// RST pin. Active low. Pulsing it forces a full hardware reset.
    reset: Reset,
    /// BUSY pin. Driven by the chip; high means "I'm busy, don't talk to me
    /// yet." We poll it in [`Self::wait_busy`].
    busy: Busy,
    /// Source of `delay_ms` calls. Used for reset timing and the BUSY poll
    /// interval. Owned (not borrowed) because we need to call `&mut` methods
    /// on it from inside our own `&mut self` methods.
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
    /// Construct a driver. Does **not** touch the chip yet — call
    /// [`Self::init`] before doing anything else.
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

    /// Pulse the RST pin high → low → high to force a hardware reset.
    ///
    /// While RST is low the chip's internal logic is held in reset; on the
    /// rising edge it re-boots from OTP. After the rising edge the chip
    /// asserts BUSY for ~50ms while it runs its internal boot routines, so
    /// we wait for BUSY to drop before returning.
    pub fn reset(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.reset.set_high().map_err(DriverError::Pin)?;
        self.delay.delay_ms(10);
        self.reset.set_low().map_err(DriverError::Pin)?;
        self.delay.delay_ms(10);
        self.reset.set_high().map_err(DriverError::Pin)?;
        self.delay.delay_ms(10);
        self.wait_busy()
    }

    /// Run the full documented init sequence.
    ///
    /// After this returns the chip is ready to accept RAM writes:
    /// 1. Hardware reset.
    /// 2. Software reset (opcode `0x12`) + wait_busy.
    /// 3. Data-entry mode = row-major auto-increment (opcode `0x11`).
    /// 4. Display update control 1 = `[0x40, 0x00]` — the value tri-color
    ///    refresh actually needs on this panel (see
    ///    [`DisplayUpdateOptions`] for why this isn't `[0x00, 0x00]`).
    /// 5. Border waveform = default (opcode `0x3C`).
    /// 6. Temperature source = internal sensor (opcode `0x18`).
    /// 7. RAM window = the entire 400×300 panel (opcodes `0x44`+`0x45`).
    /// 8. RAM cursor = `(0, 0)` (opcodes `0x4E`+`0x4F`).
    pub fn init(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.reset()?;
        self.software_reset()?;

        self.set_data_entry_mode(DataEntryMode::IncrementXIncrementYXMajor)?;
        // self.set_display_update_control_1(DisplayUpdateOptions::TriColor420)?;
        // self.set_border_waveform(BorderWaveform::Default)?;
        // self.set_temperature_source(TemperatureSource::Internal)?;

        self.set_ram_window(0, 0, WIDTH - 1, HEIGHT - 1)?;
        self.set_ram_address(0, 0)?;

        Ok(())
    }

    /// Send opcode `0x12` (software reset) and wait for the chip to settle.
    ///
    /// Lighter than [`Self::reset`] because it doesn't toggle RST — the
    /// controller's command/control registers reset, but the OTP-loaded
    /// waveform LUTs in chip RAM survive. BUSY goes high for ~50ms;
    /// we wait for it.
    pub fn software_reset(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command_only(opcode::SW_RESET)?;
        self.wait_busy()
    }

    /// Send opcode `0x11` to configure how the RAM address counter advances.
    ///
    /// See [`DataEntryMode`] for the variants. For a top-to-bottom,
    /// left-to-right framebuffer dump, use
    /// [`DataEntryMode::IncrementXIncrementYXMajor`].
    pub fn set_data_entry_mode(
        &mut self,
        mode: DataEntryMode,
    ) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command_with_data(opcode::DATA_ENTRY_MODE, &[mode.byte()])
    }

    // /// Send opcode `0x21` (Display Update Control 1) — the per-plane
    // /// "how to mix RAM1 (B/W) and RAM2 (red) into the panel output" config.
    // ///
    // /// **The empirical value is not the datasheet's "Normal/Normal."**
    // /// See [`DisplayUpdateOptions`] for the full story; the short version
    // /// is that this panel needs `[0x40, 0x00]` for tri-color refresh to
    // /// actually run, and the red plane comes out bit-inverted as a side
    // /// effect (compensated for in the tricolor encoding tables).
    // pub fn set_display_update_control_1(
    //     &mut self,
    //     options: DisplayUpdateOptions,
    // ) -> CmdResult<Spi::Error, DataCommand::Error> {
    //     self.command_with_data(opcode::DISPLAY_UPDATE_CONTROL_1, &options.bytes())
    // }

    /// Send opcode `0x3C` to set the border-ring waveform.
    pub fn set_border_waveform(
        &mut self,
        waveform: BorderWaveform,
    ) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command_with_data(opcode::WRITE_BORDER, &[waveform.byte()])
    }

    /// Send opcode `0x18` to pick which thermistor drives the LUT auto-load.
    /// Use [`TemperatureSource::Internal`] on our board.
    pub fn set_temperature_source(
        &mut self,
        source: TemperatureSource,
    ) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command_with_data(opcode::TEMP_CONTROL, &[source.byte()])
    }

    /// Configure the rectangular region of RAM that subsequent writes target,
    /// using opcodes `0x44` (X range) and `0x45` (Y range).
    ///
    /// `x1`/`x2` are in **pixels**, but the X-axis hardware addresses bytes
    /// (8 pixels each), so we divide by 8 on the wire. `y1`/`y2` are in pixel
    /// rows and go out as little-endian u16 because 300 doesn't fit in a u8.
    ///
    /// Pass `(0, 0, WIDTH - 1, HEIGHT - 1)` to cover the whole panel.
    pub fn set_ram_window(
        &mut self,
        x1: u16,
        y1: u16,
        x2: u16,
        y2: u16,
    ) -> CmdResult<Spi::Error, DataCommand::Error> {
        // BUG?
        self.command_with_data(opcode::SET_RAM_X_RANGE, &[(x1 >> 3) as u8, (x2 >> 3) as u8])?;
        self.command_with_data(
            opcode::SET_RAM_Y_RANGE,
            &[y1 as u8, (y1 >> 8) as u8, y2 as u8, (y2 >> 8) as u8],
        )
    }

    pub fn set_ram_address(&mut self, x: u16, y: u16) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command_with_data(opcode::SET_RAM_X_COUNTER, &[(x >> 3) as u8])?;
        self.command_with_data(opcode::SET_RAM_Y_COUNTER, &[y as u8, (y >> 8) as u8])
    }

    pub fn start_write_ram1(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command_only(opcode::WRITE_RAM_BW)
    }

    pub fn start_write_ram2(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command_only(opcode::WRITE_RAM_RED)
    }

    pub fn write_data(&mut self, bytes: &[u8]) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.data(bytes)
    }

    pub fn flash_test(
        &mut self,
        bw_byte: u8,
        red_byte: u8,
    ) -> CmdResult<Spi::Error, DataCommand::Error> {
        const CHUNK: usize = 256;
        const TOTAL_BYTES: usize = (WIDTH as usize * HEIGHT as usize) / 8;

        self.set_ram_window(0, 0, WIDTH - 1, HEIGHT - 1)?;

        self.set_ram_address(0, 0)?;
        self.start_write_ram1()?;
        let bw_chunk = [bw_byte; CHUNK];
        let mut sent = 0;
        while sent < TOTAL_BYTES {
            let n = (TOTAL_BYTES - sent).min(CHUNK);
            self.write_data(&bw_chunk[..n])?;
            sent += n;
        }

        self.set_ram_address(0, 0)?;
        self.start_write_ram2()?;
        let red_chunk = [red_byte; CHUNK];
        let mut sent = 0;
        while sent < TOTAL_BYTES {
            let n = (TOTAL_BYTES - sent).min(CHUNK);
            self.write_data(&red_chunk[..n])?;
            sent += n;
        }

        self.refresh()
    }

    pub fn refresh(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command_with_data(opcode::DISPLAY_UPDATE_CONTROL_2, &[0xF7])?;
        // self.command_with_data(
        //     opcode::DISPLAY_UPDATE_CONTROL_2,
        //     &[UpdateSequence::Full.byte()],
        // )?;
        self.command_only(opcode::MASTER_ACTIVATE)?;
        self.wait_busy()
    }

    pub fn sleep(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command_with_data(opcode::DEEP_SLEEP, &[0x01])?;
        self.delay.delay_ms(100);
        Ok(())
    }

    /// Send a one-byte opcode with DC low. No data follows.
    fn command_only(&mut self, opcode: u8) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command(opcode)
    }

    /// Send a one-byte opcode (DC low) followed by N data bytes (DC high).
    /// This is the shape of nearly every SSD1683 command.
    pub fn command_with_data(
        &mut self,
        opcode: u8,
        data: &[u8],
    ) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.command(opcode)?;
        self.data(data)
    }

    /// Pull DC low, push one opcode byte over SPI.
    pub fn command(&mut self, cmd: u8) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.data_command.set_low().map_err(DriverError::Pin)?;
        self.spi.write(&[cmd]).map_err(DriverError::Spi)?;
        Ok(())
    }

    /// Pull DC high, push N data bytes over SPI in one transaction.
    pub fn data(&mut self, bytes: &[u8]) -> CmdResult<Spi::Error, DataCommand::Error> {
        self.data_command.set_high().map_err(DriverError::Pin)?;
        self.spi.write(bytes).map_err(DriverError::Spi)?;
        Ok(())
    }

    pub fn wait_busy(&mut self) -> CmdResult<Spi::Error, DataCommand::Error> {
        const BUSY_POLL_INTERVAL_MS: u32 = 10;
        const BUSY_TIMEOUT_MS: u32 = 30_000;
        let mut waited_ms: u32 = 0;
        while self.busy.is_high().map_err(DriverError::Pin)? {
            self.delay.delay_ms(BUSY_POLL_INTERVAL_MS);
            waited_ms = waited_ms.saturating_add(BUSY_POLL_INTERVAL_MS);
            if waited_ms >= BUSY_TIMEOUT_MS {
                info!("(SSD1683) wait_busy TIMEOUT after {}ms", waited_ms);
                return Err(DriverError::BusyTimeout);
            }
        }
        info!("(SSD1683) wait_busy released after {}ms", waited_ms);
        Ok(())
    }
}
