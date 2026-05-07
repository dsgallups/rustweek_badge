use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures_timer::Delay;
use std::time::Duration;
use uuid::Uuid;

pub const DEVICE_NAME: &str = "nameyourbadge";
pub const SERVICE_UUID: Uuid = Uuid::from_u128(12_897_126_749_781_238);
pub const RX_CHAR_UUID: Uuid = Uuid::from_u128(12_847_126_749_781_238);

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

pub async fn scan_for_badge(adapter: &Adapter, dwell: Duration) -> Result<Vec<Discovered>, String> {
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| e.to_string())?;
    Delay::new(dwell).await;
    let _ = adapter.stop_scan().await;

    let mut found = Vec::new();
    for p in adapter.peripherals().await.map_err(|e| e.to_string())? {
        let Ok(Some(props)) = p.properties().await else {
            continue;
        };
        if props.local_name.as_deref() != Some(DEVICE_NAME) {
            continue;
        }
        found.push(Discovered {
            peripheral: p,
            name: props.local_name.unwrap_or_default(),
            rssi: props.rssi,
        });
    }
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

pub async fn write_rgb(p: &Peripheral, r: u8, g: u8, b: u8) -> Result<(), String> {
    let chars = p.characteristics();
    let rx = chars
        .iter()
        .find(|c| c.uuid == RX_CHAR_UUID)
        .ok_or_else(|| "rx characteristic not found".to_string())?;
    p.write(rx, &[0x01, r, g, b], WriteType::WithoutResponse)
        .await
        .map_err(|e| e.to_string())
}
