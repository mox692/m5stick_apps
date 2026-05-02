#![no_std]
#![no_main]
#![deny(clippy::large_stack_frames)]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Level, Output, OutputConfig},
    main,
    spi::master::{Config, Spi},
    time::Rate,
};
use log::info;

extern crate alloc;

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::{Builder, options::ColorOrder};

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

    // M5StickC Plus2 display pins
    let sck = peripherals.GPIO13;
    let mosi = peripherals.GPIO15;
    let _back_light = Output::new(peripherals.GPIO27, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO14, Level::Low, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO12, Level::High, OutputConfig::default());
    let cs = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());

    // Initialize SPI (40 MHz)
    let spi = Spi::new(
        peripherals.SPI2,
        Config::default().with_frequency(Rate::from_mhz(40)),
    )
    .expect("Failed to create SPI")
    .with_sck(sck)
    .with_mosi(mosi);

    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let mut delay = Delay::new();

    info!("Initializing display...");

    // Buffer for SPI interface
    let mut buffer = [0u8; 320];
    let di = mipidsi::interface::SpiInterface::new(spi_device, dc, &mut buffer);

    let mut display = Builder::new(mipidsi::models::ST7789, di)
        .reset_pin(rst)
        .display_size(135, 240)
        .display_offset(52, 40)
        .color_order(ColorOrder::Rgb)
        .init(&mut delay)
        .unwrap();

    info!("Drawing to display...");

    let colors = [Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE, Rgb565::YELLOW];
    let mut color_idx = 0;

    loop {
        // Fill screen with current color
        Rectangle::new(Point::new(0, 0), Size::new(240, 135))
            .into_styled(PrimitiveStyle::with_fill(colors[color_idx]))
            .draw(&mut display)
            .unwrap();

        info!("Display updated with color {}", color_idx);

        // Move to next color
        color_idx = (color_idx + 1) % colors.len();

        delay.delay_millis(1000);
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}
