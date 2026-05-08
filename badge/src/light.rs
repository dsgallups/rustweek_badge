use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use esp_hal::{
    Blocking,
    gpio::Level,
    peripherals::{GPIO9, RMT},
    rmt::{Channel as RmtChannel, PulseCode, Rmt, Tx, TxChannelConfig, TxChannelCreator},
    time::Rate,
};
use shared::LightCommand;

pub static LIGHT_CHANNEL: Channel<CriticalSectionRawMutex, LightCommand, 4> = Channel::new();

#[embassy_executor::task]
pub async fn run_light(rmt: RMT<'static>, gpio9: GPIO9<'static>) {
    let rmt = Rmt::new(rmt, Rate::from_mhz(80)).unwrap();

    let mut buf = [PulseCode::end_marker(); 25];
    let color = encode(&mut buf, 4, 0, 2);

    let mut tx = rmt
        .channel0
        .configure_tx(&TxChannelConfig::default().with_clk_divider(1))
        .unwrap()
        .with_pin(gpio9);
    tx = set_color(tx, &color);

    loop {
        let value = LIGHT_CHANNEL.receive().await;
        tx = set_color(tx, encode(&mut buf, value.r, value.g, value.b));
    }

    // Timer::after(Duration::from_secs(5)).await;
    // set_color(tx, &off);
}

fn set_color<'c>(
    tx: RmtChannel<'c, Blocking, Tx>,
    color: &[PulseCode; 25],
) -> RmtChannel<'c, Blocking, Tx> {
    match tx.transmit(color) {
        Ok(txn) => match txn.wait() {
            Ok(c) | Err((_, c)) => c,
        },
        Err((_, c)) => c,
    }
}

const T0H: u16 = 30;
const T0L: u16 = 70;
const T1H: u16 = 70;
const T1L: u16 = 30;

fn encode(buf: &mut [PulseCode; 25], r: u8, g: u8, b: u8) -> &[PulseCode; 25] {
    // GRB, MSB
    let grb = ((g as u32) << 16) | ((r as u32) << 8) | (b as u32);

    for i in 0..24 {
        let bit = (grb >> (23 - i)) & 1;
        buf[i] = if bit == 1 {
            PulseCode::new(Level::High, T1H, Level::Low, T1L)
        } else {
            PulseCode::new(Level::High, T0H, Level::Low, T0L)
        };
    }
    buf
}
