use trouble_host::attribute::Uuid;

/// Set your own for building purposes. we should have different ones
pub const BLUETOOTH_MAC_ADDRESS: [u8; 6] = [0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff];
pub const DEVICE_NAME: &str = "nameyourbadge";

/// Dont change these
pub const CONNECTIONS_MAX: usize = 1;
pub const L2CAP_CHANNELS_MAX: usize = 2;

pub const SERVICE_UUID: u128 = 12897126749781238;
pub const RX_CHAR_UUID: u128 = 12847126749781238;
