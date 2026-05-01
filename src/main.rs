#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod epd;
mod testing;

use core::cell::RefCell;

use crate::epd::display420::Display420Mono;
use crate::epd::sram23k256::Sram23k256;
use crate::epd::ssd1683::{HEIGHT, Ssd1683, WIDTH};
use bevy_app::{App, Update};
use bevy_ecs::resource::Resource;
use bevy_ecs::system::ResMut;
use defmt::info;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Line, PrimitiveStyle};
use embedded_hal_bus::spi::RefCellDevice;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::main;
use esp_hal::rmt::{PulseCode, Rmt, TxChannelConfig, TxChannelCreator};
use esp_hal::spi::Mode as SpiMode;
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::time::{Duration, Instant, Rate};
use esp_hal::timer::timg::TimerGroup;
use esp_radio::ble::controller::BleConnector;
use panic_rtt_target as _;

use crate::testing::test_display;

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32c6 -o esp32c6-mini-1 -o alloc -o unstable-hal -o wifi -o ble-bleps -o probe-rs -o defmt -o panic-rtt-target -o zed -o nightly-aarch64-apple-darwin

    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO4
    // - GPIO5
    // - GPIO8
    // - GPIO9
    // - GPIO15
    // These GPIO pins are in use by some feature of the module and should not be used.
    let _ = peripherals.GPIO24;
    let _ = peripherals.GPIO25;
    let _ = peripherals.GPIO26;
    let _ = peripherals.GPIO27;
    let _ = peripherals.GPIO28;
    let _ = peripherals.GPIO29;
    let _ = peripherals.GPIO30;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    let _connector = BleConnector::new(peripherals.BT, Default::default());

    let neopixel_power = peripherals.GPIO20;
    let mut power = Output::new(neopixel_power, Level::High, OutputConfig::default());
    power.set_high();

    // initialized by physical order
    //
    // GPIO6 - A2
    let epd_busy = Input::new(
        peripherals.GPIO6,
        InputConfig::default().with_pull(Pull::None),
    );
    // GPIO5 - A3
    let sram_cs = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());

    // two pin gap
    info!("EPD: configuring SPI2");
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(4))
            .with_mode(SpiMode::_0),
    )
    .expect("SPI2 config")
    .with_sck(peripherals.GPIO21)
    .with_mosi(peripherals.GPIO22)
    .with_miso(peripherals.GPIO23);

    let spi_bus = RefCell::new(spi);
    let epd_dc = Output::new(peripherals.GPIO17, Level::High, OutputConfig::default());
    let epd_cs = Output::new(peripherals.GPIO16, Level::High, OutputConfig::default());

    // this one is on the right hand side
    let epd_rst = Output::new(peripherals.GPIO18, Level::High, OutputConfig::default());

    let epd_spi = RefCellDevice::new(&spi_bus, epd_cs, Delay::new()).expect("epd device");
    let sram_spi = RefCellDevice::new(&spi_bus, sram_cs, Delay::new()).expect("sram device");

    let mut sram = Sram23k256::new(sram_spi);
    if let Err(e) = sram.set_sequential_mode() {
        defmt::error!("SRAM seq mode failed: {:?}", e);
    } else {
        info!("SRAM seq mode OK");
    }

    let mut epd = Ssd1683::new(epd_spi, epd_dc, epd_rst, epd_busy, Delay::new());
    match epd.init() {
        Ok(()) => info!("EPD init OK ({} x {})", WIDTH, HEIGHT),
        Err(e) => defmt::error!("EPD init failed: {:?}", e),
    }

    let mut display = Display420Mono::new(sram);

    let mut app = App::new();
    app.init_resource::<Counter>();
    app.add_systems(Update, test_schedule);

    let color = encode(4, 0, 2);

    let off = encode(0, 0, 0);

    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).unwrap();

    let mut tx = rmt
        .channel0
        .configure_tx(&TxChannelConfig::default().with_clk_divider(1))
        .unwrap()
        .with_pin(peripherals.GPIO9);

    let mut transmit_color = true;

    loop {
        let color = if transmit_color { &color } else { &off };
        tx = match tx.transmit(color) {
            Ok(txn) => match txn.wait() {
                Ok(c) | Err((_, c)) => c,
            },
            Err((_, c)) => c,
        };
        info!("looping around");
        test_display(&mut display, &mut epd);
        // neopixel.toggle();
        // info!("Hello world!");
        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(500) {}
        // app.update();
        transmit_color = !transmit_color;
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
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

#[derive(Resource, Default)]
pub struct Counter(u32);

fn test_schedule(mut counter: ResMut<Counter>) {
    info!("in ecs sched :) {}", counter.0);
    counter.0 += 1;
}
