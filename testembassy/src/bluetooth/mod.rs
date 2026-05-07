mod server;

use crate::{
    CONNECTIONS_MAX, L2CAP_CHANNELS_MAX,
    bluetooth::server::Server,
    consts::{BLUETOOTH_DEVICE_ADDRESS, DEVICE_NAME},
};
use bt_hci::{controller::ExternalController, uuid::appearance};
use defmt::panic;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use esp_hal::peripherals::BT;
use esp_radio::ble::controller::BleConnector;
use static_cell::StaticCell;
use trouble_host::{
    Address, Host, HostResources, Stack,
    gap::{GapConfig, PeripheralConfig},
    peripheral::Peripheral,
    prelude::{DefaultPacketPool, Runner},
};

type BleController = ExternalController<BleConnector<'static>, 20>;

pub async fn init(spawner: &Spawner, bluetooth: BT<'static>) {
    // BLE controller stuff. This is the the HCI "Host-Controller Interface" lower half.
    let transport = BleConnector::new(bluetooth, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, 20>::new(transport);

    static RESOURCES: StaticCell<
        HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX>,
    > = StaticCell::new();
    static STACK: StaticCell<Stack<'static, BleController, DefaultPacketPool>> = StaticCell::new();
    static SERVER: StaticCell<Server<'static>> = StaticCell::new();

    // Trouble host stack
    let resources = RESOURCES.init(HostResources::new());
    let stack = STACK.init(
        trouble_host::new(ble_controller, resources)
            .set_random_address(Address::random(BLUETOOTH_DEVICE_ADDRESS)),
    );

    let Host {
        mut peripheral,
        mut runner,
        ..
    } = stack.build();

    let server = SERVER.init(
        Server::new_with_config(GapConfig::Peripheral(PeripheralConfig {
            name: DEVICE_NAME,
            appearance: &appearance::UNKNOWN,
        }))
        .unwrap(),
    );

    spawner.spawn(bluetooth_runner_task(runner).unwrap());
    spawner.spawn(bluetooth_app_task(peripheral, server).unwrap());

    // join(
    //     async {
    //         loop {
    //             if let Err(e) = runner.run().await {
    //                 panic!("BLE RUNNER ERROR {}", e);
    //             }
    //         }
    //     },
    //     async {
    //         //todo
    //     },
    // )
    // .await;

    // GATT (Generic Attribute File) server. This defines how BLE devices exchange data once
    // they're connected. It holds the data and exposes it to other devices.
    //
    // Servers host the data and are given commands. clients push data onto the server.

    // let server = Server
}

#[embassy_executor::task]
async fn bluetooth_runner_task(
    mut runner: Runner<'static, ExternalController<BleConnector<'static>, 20>, DefaultPacketPool>,
) {
    if let Err(e) = runner.run().await {
        panic!("BLE RUNNER ERROR {}", e);
    }
}

#[embassy_executor::task]
async fn bluetooth_app_task(
    mut peripheral: Peripheral<'static, BleController, DefaultPacketPool>,
    server: &'static Server<'static>,
) {
    //todo
}

// #[embassy_executor::task]
// pub async fn listen_to_bluetooth(bluetooth: BT<'static>) {
//     let transport = BleConnector::new(bluetooth, Default::default()).unwrap();
//     let ble_controller = ExternalController::<_, 20>::new(transport);
//     let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
//         HostResources::new();

//     let stack = trouble_host::new(ble_controller, &mut resources)
//         .set_random_address(Address::random(BLUETOOTH_DEVICE_ADDRESS));

//     let host = stack.build();

//     // let host = stack.
// }
