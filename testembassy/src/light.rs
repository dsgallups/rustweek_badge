use embassy_time::{Duration, Timer};
use esp_hal::{
    gpio::Level,
    peripherals::{GPIO9, RMT},
    rmt::{PulseCode, Rmt, TxChannelConfig, TxChannelCreator},
    time::Rate,
};

#[embassy_executor::task]
pub async fn run_light(rmt: RMT<'static>, gpio9: GPIO9<'static>) {
    let rmt = Rmt::new(rmt, Rate::from_mhz(80)).unwrap();

    let color = encode(4, 0, 2);

    let off = encode(0, 0, 0);

    let mut transmit_color = true;

    let mut tx = rmt
        .channel0
        .configure_tx(&TxChannelConfig::default().with_clk_divider(1))
        .unwrap()
        .with_pin(gpio9);

    loop {
        let color = if transmit_color { &color } else { &off };
        tx = match tx.transmit(color) {
            Ok(txn) => match txn.wait() {
                Ok(c) | Err((_, c)) => c,
            },
            Err((_, c)) => c,
        };
        Timer::after(Duration::from_millis(500)).await;
        transmit_color = !transmit_color;
    }
}

const T0H: u16 = 30;
const T0L: u16 = 70;
const T1H: u16 = 70;
const T1L: u16 = 30;

fn encode(r: u8, g: u8, b: u8) -> [PulseCode; 25] {
    // GRB, MSB
    let grb = ((g as u32) << 16) | ((r as u32) << 8) | (b as u32);

    let mut buf = [PulseCode::end_marker(); 25];

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
