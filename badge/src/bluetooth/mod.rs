use crate::{
    CONNECTIONS_MAX, L2CAP_CHANNELS_MAX,
    consts::{BLUETOOTH_DEVICE_ADDRESS, DEVICE_NAME},
    display::DRAW_CHANNEL,
    light::LIGHT_CHANNEL,
};
use alloc::string::ToString;
use bt_hci::{controller::ExternalController, uuid::appearance};
use defmt::{error, info, panic};
use embassy_executor::Spawner;
use esp_hal::peripherals::BT;
use esp_radio::ble::controller::BleConnector;
use shared::{ArchivedBadgeCommand, DrawCommand, LightCommand, RX_CHAR_UUID, SERVICE_UUID};
use static_cell::StaticCell;
use trouble_host::prelude::*;

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
        peripheral, runner, ..
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
}

#[embassy_executor::task]
async fn bluetooth_runner_task(
    mut runner: Runner<'static, ExternalController<BleConnector<'static>, 20>, DefaultPacketPool>,
) {
    if let Err(e) = runner.run().await {
        panic!("BLE RUNNER ERROR {}", e);
    }
}

#[gatt_server]
pub struct Server {
    cmd_service: CmdService,
}

#[gatt_service(uuid = SERVICE_UUID)]
struct CmdService {
    #[characteristic(uuid = RX_CHAR_UUID, write_without_response)]
    rx: [u8; 32],
}

#[embassy_executor::task]
async fn bluetooth_app_task(
    mut peripheral: Peripheral<'static, BleController, DefaultPacketPool>,
    server: &'static Server<'static>,
) {
    let mut advertisement_data = [0u8; 31];

    let length = AdStructure::encode_slice(
        &[
            // means
            // 1. I'm visible indefinitely
            // 2. dont u try that classic bluetooth bullshit on me. fk u
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::CompleteLocalName(DEVICE_NAME.as_bytes()),
        ],
        &mut advertisement_data[..],
    )
    .unwrap();

    let rx_handle = server.cmd_service.rx.handle;

    loop {
        let advertiser = peripheral
            .advertise(
                &Default::default(),
                Advertisement::ConnectableScannableUndirected {
                    adv_data: &advertisement_data[..length],
                    scan_data: &[],
                },
            )
            .await
            .unwrap();
        info!("Advertising as '{}'", DEVICE_NAME);

        let conn = advertiser
            .accept()
            .await
            .unwrap()
            .with_attribute_server(server)
            .unwrap();

        info!("Connected to client!");

        loop {
            match conn.next().await {
                GattConnectionEvent::Disconnected { reason } => {
                    info!("(GATT) Disconnected: {:?}", reason);
                    break;
                }
                GattConnectionEvent::Gatt { event } => {
                    if let GattEvent::Write(w) = &event {
                        if w.handle() == rx_handle {
                            dispatch(w.data()).await;
                        }
                    }

                    info!("(GATT) Event received");
                }
                GattConnectionEvent::ConnectionParamsUpdated {
                    conn_interval: _,
                    peripheral_latency: _,
                    supervision_timeout: _,
                } => {
                    info!("(GATT) Connection params update received");
                }
                GattConnectionEvent::DataLengthUpdated {
                    max_tx_octets: _,
                    max_tx_time: _,
                    max_rx_octets: _,
                    max_rx_time: _,
                } => {
                    info!("(GATT) Data length updated");
                }
                GattConnectionEvent::PhyUpdated {
                    tx_phy: _,
                    rx_phy: _,
                } => {
                    info!("(GATT) Phy??w tf is this updated");
                }
                GattConnectionEvent::RequestConnectionParams(_params) => {
                    info!("(GATT) RequestConnectionParams recieved");
                }
            }
        }
    }

    //todo
}

async fn dispatch(bytes: &[u8]) {
    let archived = match rkyv::access::<ArchivedBadgeCommand, rkyv::rancor::Error>(bytes) {
        Ok(archived) => archived,
        Err(e) => {
            let val = e.to_string();
            error!("Cannot deserialize command! {}", val.as_str());
            return;
        }
    };

    match archived {
        ArchivedBadgeCommand::Debug => {
            info!("Debug command!");
        }
        ArchivedBadgeCommand::SetLight(light) => {
            let value = rkyv::deserialize::<LightCommand, rkyv::rancor::Error>(light).unwrap();
            LIGHT_CHANNEL.send(value).await;

            info!("Sent light command!");
        }
        ArchivedBadgeCommand::Drawing(draw_command) => {
            let value =
                rkyv::deserialize::<DrawCommand, rkyv::rancor::Error>(draw_command).unwrap();

            DRAW_CHANNEL.send(value).await;
            info!("Sent draw command!");
        }
    }
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
