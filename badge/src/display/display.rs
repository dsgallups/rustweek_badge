use core::fmt::Write;
use core::{convert::Infallible, time::Duration};

use alloc::{
    borrow::Cow,
    string::{String, ToString},
};
use defmt::{error, info};
use embassy_time::Timer;
use embedded_hal::spi::Error;
use embedded_hal_bus::spi::{DeviceError, RefCellDevice};
use esp_hal::{
    Blocking,
    delay::Delay,
    gpio::{Input, Output},
    spi::master::Spi,
};

use crate::display::drivers::{
    BorderWaveform, CmdResult, DataEntryMode, Display420Tri, DriverError, HEIGHT, Ssd1683,
    TemperatureSource, WIDTH, opcode,
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

    pub async fn debug(&mut self) -> Result<(), Failed> {
        Timer::after(embassy_time::Duration::from_secs(3)).await;
        info!("Running debug code on display!");

        self.controller.reset()?;
        self.controller.software_reset()?;

        self.controller
            .set_data_entry_mode(DataEntryMode::IncrementXIncrementYXMajor)?;
        // self.set_display_update_control_1(DisplayUpdateOptions::TriColor420)?;
        self.controller
            .set_border_waveform(BorderWaveform::Default)?;
        self.controller
            .set_temperature_source(TemperatureSource::Internal)?;

        self.controller
            .set_ram_window(0, 0, WIDTH - 1, HEIGHT - 1)?;
        self.controller.set_ram_address(0, 0)?;
        // if let Err(e) = self.controller.init() {
        //     error!("Flush to panel failed: {:?}", e);
        // }

        // self.controller().refresh()
        info!("Display initialized");

        self.controller.command_with_data(
            opcode::DISPLAY_UPDATE_CONTROL_1,
            &[0b0000_0000, 0b0000_0000],
        )?;

        // self.controller.set_display_update_control_1(
        //     crate::display::drivers::DisplayUpdateOptions::TriColor420,
        // )?;
        info!("Set control");
        info!("Flashing!");

        let wait_for = 3;

        // let mut black = true;

        // loop {
        //     if black {
        //         self.controller.flash_test(0xFF, 0x00)?;
        //         info!("Flashing black!");
        //     } else {
        //         self.controller.flash_test(0xFF, 0xFF)?;
        //         info!("Flashing red!");
        //     }
        //     self.controller.wait_busy()?;
        //     black = !black;
        // }

        let codes = [[0xFF, 0x00], [0x00, 0x00], [0xFF, 0xFF]];
        // let codes = [[0x00, 0x00], [0xFF, 0xFF]];

        for i in 0..20 {
            for code in codes {
                info!("({} {:02X}, {:02X}): FLASHING", i, code[0], code[1]);
                self.controller.flash_test(code[0], code[1])?;
                info!("({} {:02X}, {:02X}): FLASHED", i, code[0], code[1]);
                Timer::after(embassy_time::Duration::from_secs(wait_for)).await;
            }
        }

        // self.controller.sleep();

        info!("Done!");
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
