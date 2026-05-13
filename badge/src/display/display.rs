use core::convert::Infallible;
use core::fmt::Write;

use alloc::{
    borrow::Cow,
    string::{String, ToString},
};
use defmt::{error, info};
use embedded_hal::spi::Error;
use embedded_hal_bus::spi::{DeviceError, RefCellDevice};
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, Output},
    spi::master::Spi,
};

use crate::display::drivers::{CmdResult, Display420Tri, DriverError, Ssd1683};

pub struct Display<'other_io, 'spi> {
    display: Display420Tri<RefCellDevice<'other_io, Spi<'spi, Blocking>, Output<'other_io>, Delay>>,
    controller: Ssd1683<
        RefCellDevice<'other_io, Spi<'spi, Blocking>, Output<'other_io>, Delay>,
        Output<'other_io>,
        Output<'other_io>,
        Input<'other_io>,
        Delay,
    >,
}

impl<'other_io, 'spi> Display<'other_io, 'spi> {
    pub fn new(
        display: Display420Tri<
            RefCellDevice<'other_io, Spi<'spi, Blocking>, Output<'other_io>, Delay>,
        >,
        controller: Ssd1683<
            RefCellDevice<'other_io, Spi<'spi, Blocking>, Output<'other_io>, Delay>,
            Output<'other_io>,
            Output<'other_io>,
            Input<'other_io>,
            Delay,
        >,
    ) -> Self {
        Self {
            display,
            controller,
        }
    }

    pub fn display(
        &mut self,
    ) -> &mut Display420Tri<RefCellDevice<'other_io, Spi<'spi, Blocking>, Output<'other_io>, Delay>>
    {
        &mut self.display
    }

    pub fn controller(
        &mut self,
    ) -> &mut Ssd1683<
        RefCellDevice<'other_io, Spi<'spi, Blocking>, Output<'other_io>, Delay>,
        Output<'other_io>,
        Output<'other_io>,
        Input<'other_io>,
        Delay,
    > {
        &mut self.controller
    }

    pub fn init(&mut self) {
        if let Err(e) = self.controller.init() {
            error!("controller init failed: {:?}", e);
        }
    }

    pub fn flush(&mut self) {
        if let Err(e) = self.display.flush_to_panel(&mut self.controller) {
            error!("flush_to_panel failed: {:?}", e);
            return;
        }
        if let Err(e) = self.controller.refresh() {
            error!("refresh failed: {:?}", e);
        }
    }

    pub fn debug(&mut self) -> Result<(), Failed> {
        info!("Running debug code on display!");
        self.controller.init()?;
        if let Err(e) = self.controller.init() {
            error!("Flush to panel failed: {:?}", e);
        }

        // self.controller().refresh()
        info!("Display initialized");
        Ok(())
        //
    }
}

pub struct Failed(pub Cow<'static, str>);

impl<Spi, Pin> From<DriverError<Spi, Pin>> for Failed
where
    Spi: Error,
    Pin: Error,
{
    fn from(val: DriverError<Spi, Pin>) -> Self {
        match val {
            DriverError::Pin(pin) => {
                let mut value = String::new();
                _ = write!(&mut value, "{:?}", pin);

                Failed(Cow::Owned(value))
                //todo
            }
            DriverError::Spi(spi) => {
                let mut value = String::new();

                _ = write!(&mut value, "{:?}", spi);

                Failed(Cow::Owned(value))
            }
            DriverError::BusyTimeout => Failed(Cow::Borrowed("Busy Timeout")),
        }
    }
}
