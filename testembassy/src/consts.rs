use trouble_host::attribute::Uuid;

/// Set your own for building purposes. we should have different ones.
///
/// This is analogous to a MAC address.
pub const BLUETOOTH_DEVICE_ADDRESS: [u8; 6] = [0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff];
pub const DEVICE_NAME: &str = "nameyourbadge";

/// Dont change these
pub const CONNECTIONS_MAX: usize = 1;

/// The Logical Link Control and Adaptation Protocol (L2CAP) provides the data transport layer for BLE.
/// GATT and other protocols run on top of L2CAP.
pub const L2CAP_CHANNELS_MAX: usize = 2;
