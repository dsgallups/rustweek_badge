use defmt::error;
use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, Output},
    spi::master::Spi,
};

use crate::display::drivers::{Display420Tri, Ssd1683};

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
}
