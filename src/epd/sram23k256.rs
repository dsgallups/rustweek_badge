use embedded_hal::spi::{Operation, SpiDevice};

const CMD_WRMR: u8 = 0x01;
const CMD_WRITE: u8 = 0x02;
const CMD_READ: u8 = 0x03;
const MODE_SEQUENTIAL: u8 = 0x40;

pub struct Sram23k256<SPI> {
    spi: SPI,
}

impl<SPI: SpiDevice> Sram23k256<SPI> {
    pub fn new(spi: SPI) -> Self {
        Self { spi }
    }

    pub fn set_sequential_mode(&mut self) -> Result<(), SPI::Error> {
        self.spi.write(&[CMD_WRMR, MODE_SEQUENTIAL])
    }

    pub fn write_byte(&mut self, addr: u16, val: u8) -> Result<(), SPI::Error> {
        self.spi
            .write(&[CMD_WRITE, (addr >> 8) as u8, addr as u8, val])
    }

    pub fn read_byte(&mut self, addr: u16) -> Result<u8, SPI::Error> {
        let header = [CMD_READ, (addr >> 8) as u8, addr as u8];
        let mut out = [0u8; 1];
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Read(&mut out)])?;
        Ok(out[0])
    }

    pub fn write_bulk(&mut self, addr: u16, data: &[u8]) -> Result<(), SPI::Error> {
        let header = [CMD_WRITE, (addr >> 8) as u8, addr as u8];
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Write(data)])
    }

    pub fn read_bulk(&mut self, addr: u16, out: &mut [u8]) -> Result<(), SPI::Error> {
        let header = [CMD_READ, (addr >> 8) as u8, addr as u8];
        self.spi
            .transaction(&mut [Operation::Write(&header), Operation::Read(out)])
    }
}
