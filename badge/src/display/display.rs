use core::cell::RefCell;

use defmt::info;
use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    peripherals::{GPIO5, GPIO6, GPIO16, GPIO17, GPIO18, GPIO21, GPIO22, GPIO23, SPI2},
    spi::{
        Mode as SpiMode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};

use crate::display::drivers::{
    display420tri::Display420Tri, sram23k256::Sram23k256, ssd1683::Ssd1683,
};

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

    pub fn do_thing(&self) {}
}
