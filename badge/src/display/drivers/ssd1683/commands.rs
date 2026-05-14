//! SSD1683 command opcodes and the parameter enums that go with them.
//!
//! The SSD1683 talks SPI in a strict pattern: pull DC low, send one **opcode**
//! byte, pull DC high, send zero-or-more **data** bytes that parameterize the
//! opcode. The data bytes are *not* free-form — each command has its own
//! layout, and most layouts pack several small fields into one or two bytes.
//!
//! Rather than scatter `0x03`, `0x80`, `0x05` etc. across the driver, every
//! parameter that's meaningful enough to name lives here as an enum. Each enum
//! exposes a tiny `pub(super) fn byte(self) -> u8` (or `bytes(...)`) so the
//! driver can write `set_data_entry_mode(DataEntryMode::IncrementXIncrementYXMajor)`
//! and let this module worry about the encoding.
//!
//! Cross-reference: SSD1683 datasheet, "Command Table" section.
//! <https://www.buydisplay.com/download/ic/SSD1683.pdf>

// use embedded_hal::{
//     delay::DelayNs,
//     digital::{InputPin, OutputPin},
//     spi::SpiDevice,
// };

// use crate::display::drivers::{CmdResult, Ssd1683};

// pub enum SsdOperation {
//     DeepSleep,
//     DataEntryMode(DataEntryMode),
//     SoftwareReset,
//     TempControl,
//     MasterActivate,
//     DisplayUpdateControl1(DisplayUpdateOptions),
//     SetTemperatureSource(TemperatureSource),
//     DisplayUpdateControl2,
//     /// For Black and White Plane
//     WriteRam1,
//     /// For Red Plane
//     WriteRam2,
//     WriteBorder(BorderWaveform),
//     /// `[(x1 >> 3) as u8, (x2 >> 3) as u8]`
//     SetRamXRange([u8; 2]),
//     /// `[y1 as u8, (y1 >> 8) as u8, y2 as u8, (y2 >> 8) as u8]`
//     SetRamYRange([u8; 4]),
//     /// `[(x >> 3) as u8]`
//     SetRamXCounter(u8),
//     /// `[y as u8, (y >> 8) as u8]`
//     SetRamYCounter([u8; 2]),
// }

// impl SsdOperation {
//     pub fn run<Spi, DataCommand, Reset, Busy, Delay>(
//         &self,
//         spi: &mut Ssd1683<Spi, DataCommand, Reset, Busy, Delay>,
//     ) -> CmdResult<Spi::Error, DataCommand::Error>
//     where
//         Spi: SpiDevice,
//         DataCommand: OutputPin,
//         Reset: OutputPin<Error = DataCommand::Error>,
//         Busy: InputPin<Error = DataCommand::Error>,
//         Delay: DelayNs,
//     {
//         use SsdOperation as O;
//         match self {
//             O::DeepSleep => {
//                 spi.command(0x10)?;
//             }
//             O::DataEntryMode(mode) => {
//                 spi.command_with_data(0x11, &[mode.byte()])?;
//             }
//             O::DisplayUpdateControl1(opts) => {
//                 spi.command_with_data(0x21, &opts.bytes())?;
//             }
//             O::SetTemperatureSource(source) => {
//                 spi.command_with_data(0x18, &[source.byte()])?;
//             }

//             O::WriteBorder(wave_form) => {
//                 spi.command_with_data(0x3C, &[wave_form.byte()])?;
//             }
//             O::SetRamXRange(bytes) => {
//                 spi.command_with_data(opcode::SET_RAM_X_RANGE, bytes.as_slice())?;
//             }
//             O::SetRamYRange(bytes) => {
//                 spi.command_with_data(opcode::SET_RAM_Y_RANGE, bytes.as_slice())?;
//             }
//             O::SetRamXCounter(byte) => {
//                 spi.command_with_data(opcode::SET_RAM_X_COUNTER, &[*byte])?;
//             }
//             O::SetRamYCounter(bytes) => {
//                 spi.command_with_data(opcode::SET_RAM_X_COUNTER, bytes.as_slice())?;
//             }
//             O::SoftwareReset => {
//                 spi.command(0x12)?;
//             }
//             O::WriteRam1 => {
//                 spi.command(opcode::WRITE_RAM_BW)?;
//             }
//             O::WriteRam2 => {
//                 spi.command(opcode::WRITE_RAM_RED)?;
//             }

//             _ => todo!(),
//         }
//         Ok(())
//     }
// }

/// Single-byte command opcodes we send to the SSD1683.
///
/// Every entry here is an opcode you'll see in the datasheet's command table,
/// keyed by name. The numbers themselves are stable across the SSD168x
/// family, so anything you see in GxEPD2 / Adafruit reference code with the
/// same hex value means the same thing here.
pub mod opcode {
    /// `0x10` — Deep sleep. Followed by 1 data byte (`0x01` = mode 1 deep
    /// sleep). After this the chip ignores everything until you pulse the
    /// RST pin.
    pub const DEEP_SLEEP: u8 = 0x10;

    /// `0x11` — Data Entry Mode. Followed by 1 data byte that configures
    /// how the RAM address counter auto-increments after each pixel byte.
    /// See [`super::DataEntryMode`].
    pub const DATA_ENTRY_MODE: u8 = 0x11;

    /// `0x12` — Software reset. No data. Resets command/control registers
    /// to power-on defaults *without* losing OTP-loaded waveform LUTs.
    /// BUSY goes high while it runs; wait for it to drop.
    pub const SW_RESET: u8 = 0x12;

    /// `0x18` — Temperature sensor source select. Followed by 1 data byte.
    /// The chip uses temperature to pick the right waveform LUT — colder
    /// ink moves slower and needs longer drive. See [`super::TemperatureSource`].
    pub const TEMP_CONTROL: u8 = 0x18;

    /// `0x20` — Master Activate. The "go" button. Runs whichever update
    /// sequence steps were configured by [`DISPLAY_UPDATE_CONTROL_2`]
    /// in the immediately prior command. BUSY stays high until the
    /// refresh waveform finishes.
    pub const MASTER_ACTIVATE: u8 = 0x20;

    /// `0x21` — Display Update Control 1. Followed by 2 data bytes that
    /// control (byte 1) how the controller mixes RAM1 (B/W plane) and RAM2
    /// (red plane) before driving the panel, and (byte 2) the source-output
    /// range. See [`super::RamOption::pack`].
    pub const DISPLAY_UPDATE_CONTROL_1: u8 = 0x21;

    /// `0x22` — Display Update Control 2. Followed by 1 data byte that is
    /// a **bitmask** of which steps the chip should run on the next
    /// [`MASTER_ACTIVATE`]. See [`super::UpdateSequence`].
    pub const DISPLAY_UPDATE_CONTROL_2: u8 = 0x22;

    /// `0x24` — Write RAM 1 (black/white plane). Followed by N data bytes;
    /// the chip stores them at the current RAM address counter (set by
    /// `SET_RAM_X_COUNTER` / `SET_RAM_Y_COUNTER`) and auto-increments
    /// according to the current [`DATA_ENTRY_MODE`].
    pub const WRITE_RAM_BW: u8 = 0x24;

    /// `0x26` — Write RAM 2. On a tri-color panel this is the **red**
    /// plane; in mono partial-refresh schemes it's the previous-frame
    /// reference. Same streaming model as [`WRITE_RAM_BW`].
    pub const WRITE_RAM_RED: u8 = 0x26;

    /// `0x3C` — Border waveform. Followed by 1 data byte that picks the
    /// waveform driven into the panel's 1-pixel-wide border (VBD pin).
    /// See [`super::BorderWaveform`].
    pub const WRITE_BORDER: u8 = 0x3C;

    /// `0x44` — Set RAM X address range (start, end). Followed by 2 data
    /// bytes, **each in byte units** (1 byte = 8 horizontal pixels). For
    /// the 400-pixel-wide panel, end = 49 covers the full width.
    pub const SET_RAM_X_RANGE: u8 = 0x44;

    /// `0x45` — Set RAM Y address range (start_lo, start_hi, end_lo, end_hi).
    /// Followed by 4 data bytes; Y is in pixel rows and the range can exceed
    /// 255 on a 300-row panel, so it's little-endian u16 pairs.
    pub const SET_RAM_Y_RANGE: u8 = 0x45;

    /// `0x4E` — Set RAM X address counter (current write cursor). Followed
    /// by 1 byte, in the same units as [`SET_RAM_X_RANGE`].
    pub const SET_RAM_X_COUNTER: u8 = 0x4E;

    /// `0x4F` — Set RAM Y address counter (current write cursor). Followed
    /// by 2 bytes, little-endian u16, in pixel rows.
    pub const SET_RAM_Y_COUNTER: u8 = 0x4F;
}

/// How the RAM address counter advances after each byte you write.
///
/// The 4.2" panel is a 400×300 grid of pixels packed 8-per-byte on the X axis.
/// After you write a byte to RAM, the chip can either step the cursor along X,
/// along Y, or stop. The data byte encodes three independent choices:
///
/// | Bit | Field |
/// | --- | --- |
/// | 0 | Y increment direction (1 = increment, 0 = decrement) |
/// | 1 | X increment direction (1 = increment, 0 = decrement) |
/// | 2 | Address-update axis (0 = X-major; X advances first, Y on wrap) |
///
/// In practice you almost always want plain row-major increment, which is
/// what the [`Self::IncrementXIncrementYXMajor`] variant encodes (`0b011 = 0x03`).
///
/// Cross-reference: datasheet, command `0x11`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataEntryMode {
    /// X and Y both increment; X advances first (row-major). Encodes `0x03`.
    /// This is the option you want when streaming a flat framebuffer top-to-bottom,
    /// left-to-right.
    IncrementXIncrementYXMajor,
    /// X decrements, Y increments; X-major. Encodes `0x01`. Useful when the
    /// panel is mounted rotated 180°.
    DecrementXIncrementYXMajor,
    /// X increments, Y decrements; X-major. Encodes `0x02`. Useful for
    /// bottom-up rendering.
    IncrementXDecrementYXMajor,
    /// X and Y both decrement; X-major. Encodes `0x00`. Mirror + flip.
    DecrementXDecrementYXMajor,
}

impl DataEntryMode {
    pub(super) fn byte(self) -> u8 {
        match self {
            Self::DecrementXDecrementYXMajor => 0x00,
            Self::DecrementXIncrementYXMajor => 0x01,
            Self::IncrementXDecrementYXMajor => 0x02,
            Self::IncrementXIncrementYXMajor => 0x03,
        }
    }
}

/// Which thermistor the chip reads to pick its refresh waveform.
///
/// The chip auto-loads a temperature-tuned LUT before each full refresh
/// (when bit 5 of `DISPLAY_UPDATE_CONTROL_2` is set — the `Read temp sensor`
/// step). It needs to know where to read the temperature from.
///
/// On the Adafruit 6381 and most e-paper modules, the built-in sensor inside
/// the controller chip is wired correctly and there's no external thermistor,
/// so [`Self::Internal`] is the right choice.
///
/// Cross-reference: datasheet, command `0x18`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperatureSource {
    /// Use the temperature sensor built into the SSD1683 die. Encodes `0x80`.
    Internal,
    /// Read temperature over the chip's I²C master from an external
    /// thermistor. Encodes `0x48`. Requires extra hardware not present on
    /// our breakout.
    ExternalI2c,
}

impl TemperatureSource {
    pub(super) fn byte(self) -> u8 {
        match self {
            Self::Internal => 0x80,
            Self::ExternalI2c => 0x48,
        }
    }
}

/// Which waveform drives the 1-pixel-wide border ring around the active area
/// during a refresh, picked by opcode `0x3C`.
///
/// The border is a separate driver output (the VBD pin), not part of either
/// RAM plane, so it has its own waveform. Most panels look correct with
/// [`Self::Default`] — that's the value the old driver used and is what the
/// Adafruit example code recommends for this module.
///
/// Cross-reference: datasheet, command `0x3C`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderWaveform {
    /// `0x05` — the "GS transition / follow LUT1 / VBD level VSS" default
    /// that works on the Adafruit 6381. Other variants exist in the
    /// datasheet but we haven't needed them.
    Default,
}

impl BorderWaveform {
    pub(super) fn byte(self) -> u8 {
        match self {
            Self::Default => 0x05,
        }
    }
}

/// Which steps the chip should run on the next `MASTER_ACTIVATE`, configured
/// by opcode `0x22` (Display Update Control 2).
///
/// The parameter byte is a **bitmask** of nine refresh-pipeline steps that
/// the chip walks through in order: enable clock → enable analog → read
/// temperature → load Mode 1 LUT → load Mode 2 LUT → DISPLAY → disable
/// analog → disable OSC. Different bitmasks give you full refresh, partial
/// refresh, or various lifecycle subsets.
///
/// For now we only expose [`Self::Full`] (`0xF7` — everything on except
/// the Mode 2 LUT load); partial refresh + LUT-loading variants will be
/// added when we wire up partial refresh.
///
/// Cross-reference: datasheet, command `0x22`, bit-field table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateSequence {
    /// `0xF7 = 0b1111_0111` — enable HV, enable analog, read temp, load
    /// Mode 1 LUT (full-refresh waveform), DISPLAY, disable analog,
    /// disable OSC. Mode 2 LUT load is *off*. This is the standard
    /// "do a full refresh" recipe.
    Full,
}

impl UpdateSequence {
    pub(super) fn byte(self) -> u8 {
        match self {
            Self::Full => 0xF7,
        }
    }
}
