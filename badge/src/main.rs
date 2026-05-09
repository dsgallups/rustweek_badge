#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

mod bluetooth;
mod command;
mod consts;
mod display;
mod light;

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Input, Level, Output, OutputConfig};
use esp_hal::timer::timg::TimerGroup;
use panic_rtt_target as _;

use crate::display::DisplayPins;

extern crate alloc;

pub const CONNECTIONS_MAX: usize = 1;
pub const L2CAP_CHANNELS_MAX: usize = 1;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // // generator version: 1.3.0
    // // generator parameters: --chip esp32c6 -o unstable-hal -o alloc -o wifi -o embassy -o ble-trouble -o probe-rs -o defmt -o panic-rtt-target -o zed -o nightly-aarch64-apple-darwin

    // // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    // info!("Embassy initialized!");

    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");

    let neopixel_power = peripherals.GPIO20;
    let mut power = Output::new(neopixel_power, Level::High, OutputConfig::default());
    power.set_high();

    spawner.spawn(light::run_light(peripherals.RMT, peripherals.GPIO9).unwrap());

    let display_pins = DisplayPins {
        paper_display_busy: peripherals.GPIO6,
        ram_chip_select: peripherals.GPIO5,
        spi_2: peripherals.SPI2,
        sck: peripherals.GPIO21,
        mosi: peripherals.GPIO22,
        miso: peripherals.GPIO23,
        display_data_command: peripherals.GPIO17,
        display_chip_select: peripherals.GPIO16,
        display_reset: peripherals.GPIO18,
    };

    spawner.spawn(display::run_display(display_pins).unwrap());

    bluetooth::init(&spawner, peripherals.BT).await;

    info!("All services initialized!");

    loop {
        Timer::after(Duration::from_secs(60)).await;
        info!("Ran for a minute!");
    }
}
