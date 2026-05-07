use crate::consts::*;
use trouble_host::prelude::*;

#[gatt_server]
pub struct Server {
    cmd_service: CmdService,
}

#[gatt_service(uuid = SERVICE_UUID)]
struct CmdService {
    #[characteristic(uuid = RX_CHAR_UUID, write_without_response)]
    rx: [u8; 32],
}
