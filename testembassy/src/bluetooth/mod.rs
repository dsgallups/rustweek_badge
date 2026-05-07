mod server;

use crate::{CONNECTIONS_MAX, L2CAP_CHANNELS_MAX, consts::BLUETOOTH_MAC_ADDRESS};
use bt_hci::controller::ExternalController;
use embassy_executor::Spawner;
use esp_hal::peripherals::BT;
use esp_radio::ble::controller::BleConnector;
use trouble_host::{Address, Host, HostResources, prelude::DefaultPacketPool};

pub fn init(spawner: &Spawner, bluetooth: BT<'static>) {
    // BLE controller stuff. This is the the HCI "Host-Controller Interface" lower half.
    let transport = BleConnector::new(bluetooth, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, 20>::new(transport);

    // Trouble host stack
    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let stack = trouble_host::new(ble_controller, &mut resources)
        .set_random_address(Address::random(BLUETOOTH_MAC_ADDRESS));
    let Host {
        mut peripheral,
        runner,
        central,
        ..
    } = stack.build();

    // GATT (Generic Attribute File) server. This defines how BLE devices exchange data once
    // they're connected. It holds the data and exposes it to other devices.
    //
    // Servers host the data and are given commands. clients push data onto the server.

    // let server = Server
}

#[embassy_executor::task]
pub async fn listen_to_bluetooth(bluetooth: BT<'static>) {
    let transport = BleConnector::new(bluetooth, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, 20>::new(transport);
    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();

    let stack = trouble_host::new(ble_controller, &mut resources)
        .set_random_address(Address::random(BLUETOOTH_MAC_ADDRESS));

    let host = stack.build();

    // let host = stack.
}
