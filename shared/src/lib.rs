#![no_std]

/// Dont change these.
pub const SERVICE_UUID: u128 = 12897126749781238;
pub const RX_CHAR_UUID: u128 = 12847126749781238;

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Archive)]
enum Command {
    Hello,
    SetLight(LightCommand),
}

impl Command {
    // fn parse(bytes: &[u8]) -> Self {
    //     match bytes {
    //         [0x3C, 0x3C] => Self::Hello,
    //         _ => Self::SetLight,
    //     }
    // }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Archive)]
pub struct LightCommand {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
