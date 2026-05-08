use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures_timer::Delay;
use shared::BadgeCommand;
use std::time::Duration;
use uuid::Uuid;

pub const SERVICE_UUID: Uuid = Uuid::from_u128(shared::SERVICE_UUID);
pub const RX_CHAR_UUID: Uuid = Uuid::from_u128(shared::RX_CHAR_UUID);

#[derive(Clone, Debug)]
pub struct Discovered {
    pub peripheral: Peripheral,
    pub name: String,
    pub rssi: Option<i16>,
}

pub async fn first_adapter() -> Result<Adapter, String> {
    let manager = Manager::new().await.map_err(|e| e.to_string())?;
    manager
        .adapters()
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .next()
        .ok_or_else(|| "no BLE adapter available".to_string())
}

pub async fn scan_all(adapter: &Adapter, dwell: Duration) -> Result<Vec<Discovered>, String> {
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| e.to_string())?;
    Delay::new(dwell).await;
    let _ = adapter.stop_scan().await;

    let mut found = Vec::new();
    for p in adapter.peripherals().await.map_err(|e| e.to_string())? {
        let props = p.properties().await.ok().flatten();
        let name = props
            .as_ref()
            .and_then(|p| p.local_name.clone())
            .unwrap_or_else(|| "(unnamed)".to_string());
        let rssi = props.as_ref().and_then(|p| p.rssi);
        found.push(Discovered {
            peripheral: p,
            name,
            rssi,
        });
    }
    found.sort_by(|a, b| b.rssi.unwrap_or(i16::MIN).cmp(&a.rssi.unwrap_or(i16::MIN)));
    Ok(found)
}

pub async fn connect(p: &Peripheral) -> Result<(), String> {
    if !p.is_connected().await.unwrap_or(false) {
        p.connect().await.map_err(|e| e.to_string())?;
    }
    p.discover_services().await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn disconnect(p: &Peripheral) -> Result<(), String> {
    p.disconnect().await.map_err(|e| e.to_string())
}

pub async fn write_command(p: &Peripheral, cmd: &BadgeCommand) -> Result<(), String> {
    let chars = p.characteristics();
    let rx = chars
        .iter()
        .find(|c| c.uuid == RX_CHAR_UUID)
        .ok_or_else(|| "rx characteristic not found".to_string())?;
    let bytes =
        rkyv::to_bytes::<rkyv::rancor::Error>(cmd).map_err(|e| format!("encode: {e}"))?;
    p.write(rx, &bytes, WriteType::WithoutResponse)
        .await
        .map_err(|e| e.to_string())
}
