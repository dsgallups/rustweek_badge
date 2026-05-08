#![no_std]

/// Dont change these.
pub const SERVICE_UUID: u128 = 12897126749781238;
pub const RX_CHAR_UUID: u128 = 12847126749781238;

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Archive)]
pub enum BadgeCommand {
    Debug,
    SetLight(LightCommand),
    Drawing(DrawCommand),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Archive)]
pub struct LightCommand {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Archive)]
pub enum Color {
    White,
    Black,
    Red,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Archive)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Archive)]
pub enum DrawCommand {
    Line {
        start: Point,
        end: Point,
        color: Color,
    },
    Clear {
        color: Color,
    },
    Debug,
    Flush,
}
