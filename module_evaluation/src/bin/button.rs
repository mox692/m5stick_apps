#![no_std]
#![no_main]
#![deny(clippy::large_stack_frames)]

use esp_backtrace as _;
use esp_hal::gpio::{Input, InputConfig, Pull};
use esp_hal::main;
use log::info;

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
    // generator version: 1.2.0

    // Use default CPU clock to avoid UART baud rate issues
    let config = esp_hal::Config::default();
    // Start the system
    let peripherals = esp_hal::init(config);

    esp_println::logger::init_logger_from_env();

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 1024);

    info!("System initialized!");

    // Configure GPIO39 as input with pull-up resistor
    let config = InputConfig::default().with_pull(Pull::Up);
    let button_a = Input::new(peripherals.GPIO39, config);
    let button_b = Input::new(peripherals.GPIO37, config);
    let button_c = Input::new(peripherals.GPIO35, config);

    let delay = esp_hal::delay::Delay::new();
    loop {
        let is_a_pressed = button_a.is_low();
        let is_b_pressed = button_b.is_low();
        let is_c_pressed = button_c.is_low();
        info!("Button A pressed: {}", is_a_pressed);
        info!("Button B pressed: {}", is_b_pressed);
        info!("Button C pressed: {}", is_c_pressed);

        delay.delay_millis(1000);
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}
