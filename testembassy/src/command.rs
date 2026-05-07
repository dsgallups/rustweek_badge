use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use shared::LightCommand;

pub static LIGHT_CHANNEL: Channel<CriticalSectionRawMutex, LightCommand, 4> = Channel::new();
