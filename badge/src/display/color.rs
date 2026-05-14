use embedded_graphics::pixelcolor::PixelColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum TriColor {
    White,
    Black,
    Red,
}

impl PixelColor for TriColor {
    type Raw = ();
}
