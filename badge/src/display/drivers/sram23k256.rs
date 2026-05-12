#![doc = r#"
Link: <https://ww1.microchip.com/downloads/en/devicedoc/22100f.pdf>

Instruction set is on Page 7, Table 2-1

"#]

use embedded_hal::spi::SpiDevice;

const WRITE: u8 = 0b0000_0010;
const READ: u8 = 0b000_0011;
// These control how the RAM is written to
const READ_STATUS: u8 = 0b000_0101;
const WRITE_STATUS: u8 = 0b000_0001;

// The commands to write into the status
// the top bits identify the mode. Page 11, Table 2-2
//
// - Page mode: The chip has 32-byte pages. You stream as many bytes as you want (ofc with the chip select
// held low). The internal address counter increments after each byets, but only within the curernt page.
// When the offset hits the end of the page, it wraps around back to the start of the same page, ovewriting,
// what was just written.
//
// - Sequential Mode: Same idea as page mode, but the counter increments across entire the entire
//   32 KiB address space.
// const BYTE_MODE: u8 = 0;
// const PAGE_MODE: u8 = 0b1000_0000;
const SEQUENTIAL_MODE: u8 = 0b0100_0000;

pub struct Sram23k256<S> {
    spi: S,
}

impl<S: SpiDevice> Sram23k256<S> {
    pub fn new(spi: S) -> Self {
        Self { spi }
    }
    /// You'll want to call this if you plan on writing data.
    ///
    /// Note that I have not provided a method for page mode, or the ability
    /// to toggle sequential mode. if we wanna publish this, we should add those methods.
    pub fn set_sequential_mode(&mut self) -> Result<(), S::Error> {
        self.spi.write(&[WRITE_STATUS, SEQUENTIAL_MODE])
    }
}
