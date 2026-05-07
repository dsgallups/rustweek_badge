use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

pub static LIGHT_CHANNEL: Channel<CriticalSectionRawMutex, Command, 4> = Channel::new();

#[derive(Debug)]
enum Command {
    Hello,
    SetLight,
}

impl Command {
    fn parse(bytes: &[u8]) -> Self {
        match bytes {
            [0x3C, 0x3C] => Self::Hello,
            _ => Self::SetLight,
        }
    }
}
