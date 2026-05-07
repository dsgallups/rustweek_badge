/// Set your own for building purposes. we should have different ones
pub const BLUETOOTH_MAC_ADDRESS: [u8; 6] = [0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff];
pub const DEVICE_NAME: &str = "nameyourbadge";

/// Dont change these
pub const CONNECTIONS_MAX: usize = 1;
pub const L2CAP_CHANNELS_MAX: usize = 2;
const SERVICE_UUID: &str = "27c5d1f0-6c50-4f9e-9d4b-3e0c8a1b2c3d";
const RX_CHAR_UUID: &str = "27c5d1f1-6c50-4f9e-9d4b-3e0c8a1b2c3d";
