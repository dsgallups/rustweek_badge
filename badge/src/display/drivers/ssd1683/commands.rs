use embedded_hal::spi::SpiDevice;

pub const SW_RESET: u8 = 0b0001_0010;

pub enum WriteCommand {
    /// Puts controller registers back to power-on defaults.
    SoftwareReset,
    UpdateDisplay {
        ram: RamOption,
    },
}
impl WriteCommand {
    pub fn command<Spi: SpiDevice>(&self, spi: &mut Spi) -> Result<(), Spi::Error> {
        use WriteCommand as C;
        let result = match self {
            C::SoftwareReset => spi.write(&[0b0001_0010]),
            C::UpdateDisplay { ram } => spi.write(&[0x21]),
        };

        result.map_err(Error::Spi)
    }
    pub fn data_command(&self) -> Set {
        use WriteCommand as C;
        match self {
            C::SoftwareReset => Set::High,
        }
    }
    pub fn mode(&self) -> Mode {
        use WriteCommand as C;
        match self {
            C::SoftwareReset => Mode::Write,
        }
    }
}

pub enum Set {
    High,
    Low,
}

pub enum Mode {
    Read,
    Write,
}

pub enum RamOption {
    Normal,
    BypassRamContent,
    InverseRamContent,
}
