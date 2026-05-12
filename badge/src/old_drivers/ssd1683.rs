use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiDevice;

use super::Error;

pub const WIDTH: u16 = 400;
pub const HEIGHT: u16 = 300;

pub mod cmd {
    pub const DRIVER_CONTROL: u8 = 0x01;
    pub const DEEP_SLEEP: u8 = 0x10;
    pub const DATA_MODE: u8 = 0x11;
    pub const SW_RESET: u8 = 0x12;
    pub const TEMP_CONTROL: u8 = 0x18;
    pub const MASTER_ACTIVATE: u8 = 0x20;
    pub const DISP_CTRL1: u8 = 0x21;
    pub const DISP_CTRL2: u8 = 0x22;
    pub const WRITE_RAM1: u8 = 0x24;
    pub const WRITE_RAM2: u8 = 0x26;
    pub const WRITE_BORDER: u8 = 0x3C;
    pub const SET_RAMXPOS: u8 = 0x44;
    pub const SET_RAMYPOS: u8 = 0x45;
    pub const SET_RAMXCOUNT: u8 = 0x4E;
    pub const SET_RAMYCOUNT: u8 = 0x4F;
}

const MONO_UPDATE_VAL: u8 = 0xF7;
const PARTIAL_UPDATE_VAL: u8 = 0xFF;

const BUSY_POLL_INTERVAL_MS: u32 = 10;
const BUSY_TIMEOUT_MS: u32 = 10_000;

pub struct Ssd1683<SPI, DC, RST, BUSY, D> {
    spi: SPI,
    dc: DC,
    rst: RST,
    busy: BUSY,
    delay: D,
}

pub type DriverError<SPI, DC> = Error<<SPI as embedded_hal::spi::ErrorType>::Error, DC>;

impl<SPI, DC, RST, BUSY, D> Ssd1683<SPI, DC, RST, BUSY, D>
where
    SPI: SpiDevice,
    DC: OutputPin,
    RST: OutputPin<Error = DC::Error>,
    BUSY: InputPin<Error = DC::Error>,
    D: DelayNs,
{
    pub fn new(spi: SPI, dc: DC, rst: RST, busy: BUSY, delay: D) -> Self {
        Self {
            spi,
            dc,
            rst,
            busy,
            delay,
        }
    }

    pub fn reset(&mut self) -> Result<(), DriverError<SPI, DC::Error>> {
        self.rst.set_high().map_err(Error::Pin)?;
        self.delay.delay_ms(10);
        self.rst.set_low().map_err(Error::Pin)?;
        self.delay.delay_ms(10);
        self.rst.set_high().map_err(Error::Pin)?;
        self.delay.delay_ms(10);
        self.wait_busy()
    }

    pub fn init(&mut self) -> Result<(), DriverError<SPI, DC::Error>> {
        self.reset()?;

        self.command(cmd::SW_RESET)?;
        self.wait_busy()?;

        self.command_with_data(cmd::DISP_CTRL1, &[0x40, 0x00])?;
        self.command_with_data(cmd::WRITE_BORDER, &[0x05])?;
        self.command_with_data(cmd::DATA_MODE, &[0x03])?;
        self.command_with_data(cmd::TEMP_CONTROL, &[0x80])?;

        self.set_ram_window(0, 0, WIDTH - 1, HEIGHT - 1)?;
        self.set_ram_address(0, 0)?;

        Ok(())
    }

    pub fn set_ram_window(
        &mut self,
        x1: u16,
        y1: u16,
        x2: u16,
        y2: u16,
    ) -> Result<(), DriverError<SPI, DC::Error>> {
        self.command_with_data(cmd::SET_RAMXPOS, &[(x1 >> 3) as u8, (x2 >> 3) as u8])?;
        self.command_with_data(
            cmd::SET_RAMYPOS,
            &[y1 as u8, (y1 >> 8) as u8, y2 as u8, (y2 >> 8) as u8],
        )?;
        Ok(())
    }

    pub fn set_ram_address(&mut self, x: u16, y: u16) -> Result<(), DriverError<SPI, DC::Error>> {
        self.command_with_data(cmd::SET_RAMXCOUNT, &[(x >> 3) as u8])?;
        self.command_with_data(cmd::SET_RAMYCOUNT, &[y as u8, (y >> 8) as u8])?;
        Ok(())
    }

    pub fn start_write_ram1(&mut self) -> Result<(), DriverError<SPI, DC::Error>> {
        self.command(cmd::WRITE_RAM1)
    }

    pub fn start_write_ram2(&mut self) -> Result<(), DriverError<SPI, DC::Error>> {
        self.command(cmd::WRITE_RAM2)
    }

    pub fn write_data(&mut self, bytes: &[u8]) -> Result<(), DriverError<SPI, DC::Error>> {
        self.data(bytes)
    }

    pub fn refresh(&mut self) -> Result<(), DriverError<SPI, DC::Error>> {
        self.command_with_data(cmd::DISP_CTRL2, &[MONO_UPDATE_VAL])?;
        self.command(cmd::MASTER_ACTIVATE)?;
        self.wait_busy()
    }

    pub fn refresh_partial(&mut self) -> Result<(), DriverError<SPI, DC::Error>> {
        self.command_with_data(cmd::DISP_CTRL2, &[PARTIAL_UPDATE_VAL])?;
        self.command(cmd::MASTER_ACTIVATE)?;
        self.wait_busy()
    }

    pub fn sleep(&mut self) -> Result<(), DriverError<SPI, DC::Error>> {
        self.command_with_data(cmd::DEEP_SLEEP, &[0x01])?;
        self.delay.delay_ms(100);
        Ok(())
    }

    fn command(&mut self, cmd: u8) -> Result<(), DriverError<SPI, DC::Error>> {
        self.dc.set_low().map_err(Error::Pin)?;
        self.spi.write(&[cmd]).map_err(Error::Spi)?;
        Ok(())
    }

    fn command_with_data(
        &mut self,
        cmd: u8,
        data: &[u8],
    ) -> Result<(), DriverError<SPI, DC::Error>> {
        self.command(cmd)?;
        self.data(data)
    }

    fn data(&mut self, bytes: &[u8]) -> Result<(), DriverError<SPI, DC::Error>> {
        self.dc.set_high().map_err(Error::Pin)?;
        self.spi.write(bytes).map_err(Error::Spi)?;
        Ok(())
    }

    fn wait_busy(&mut self) -> Result<(), DriverError<SPI, DC::Error>> {
        let mut waited_ms: u32 = 0;
        while self.busy.is_high().map_err(Error::Pin)? {
            self.delay.delay_ms(BUSY_POLL_INTERVAL_MS);
            waited_ms = waited_ms.saturating_add(BUSY_POLL_INTERVAL_MS);
            if waited_ms >= BUSY_TIMEOUT_MS {
                return Err(Error::BusyTimeout);
            }
        }
        Ok(())
    }
}
