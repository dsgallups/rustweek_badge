use crate::epd::{display420::Display420Mono, ssd1683::Ssd1683};
use defmt::{Format, info};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::SpiDevice,
};

pub fn test_display<Spi, Dc, Rst, Busy, D>(
    display: &mut Display420Mono<Spi>,
    epd: &mut Ssd1683<Spi, Dc, Rst, Busy, D>,
) where
    Spi: SpiDevice,
    Spi::Error: Format,
    Dc: OutputPin,
    Dc::Error: Format,
    Rst: OutputPin<Error = Dc::Error>,
    Busy: InputPin<Error = Dc::Error>,
    D: DelayNs,
{
    if let Err(e) = display.clear_to(BinaryColor::Off) {
        defmt::error!("EPD clear failed: {:?}", e);
    }

    if let Err(e) = Line::new(Point::new(0, 0), Point::new(399, 299))
        .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
        .draw(display)
    {
        defmt::error!("EPD draw failed: {:?}", e);
    }

    if let Err(e) = display.flush_to_panel(epd) {
        defmt::error!("EPD flush failed: {:?}", e);
    } else {
        info!("EPD flush 15000 bytes OK");
    }

    if let Err(e) = epd.refresh() {
        defmt::error!("EPD refresh failed: {:?}", e);
    } else {
        info!("EPD refresh complete");
    }

    if let Err(e) = epd.sleep() {
        defmt::error!("EPD sleep failed: {:?}", e);
    } else {
        info!("EPD diagonal drawn + sleeping");
    }
    //
}
