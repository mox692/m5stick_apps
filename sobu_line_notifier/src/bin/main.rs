#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_time::{Delay, Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::Async;
use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::rng::Rng;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use mipidsi::{
    Builder,
    interface::SpiInterface,
    models::ST7789,
    options::{ColorInversion, ColorOrder},
};
use sobu_line_notifier::ntp;
use sobu_line_notifier::timetable::{Time, get_next_trains};
use sobu_line_notifier::wifi;
use static_cell::StaticCell;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.2.0

    // let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let config = esp_hal::Config::default();
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 98768);

    // Initialize logger
    esp_println::logger::init_logger_from_env();

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    esp_rtos::start(timg0.timer0);

    log::info!("Embassy initialized!");

    // Setup DMA for SPI
    let dma_channel = peripherals.DMA_SPI2;
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = esp_hal::dma_buffers!(512);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    // Setup SPI bus for display
    let spi_bus = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(2))
            .with_mode(Mode::_0),
    )
    .unwrap()
    .with_sck(peripherals.GPIO13)
    .with_mosi(peripherals.GPIO15)
    .with_dma(dma_channel)
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    // Setup display pins
    let _back_light = Output::new(peripherals.GPIO27, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO14, Level::Low, OutputConfig::default());
    let cs = Output::new(peripherals.GPIO5, Level::High, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO12, Level::High, OutputConfig::default());

    // Create SPI device
    let spi_device: ExclusiveDevice<esp_hal::spi::master::SpiDmaBus<'_, Async>, Output<'_>, Delay> =
        ExclusiveDevice::new(spi_bus, cs, Delay).unwrap();

    // Create display interface
    static SPI_BUF: StaticCell<[u8; 512]> = StaticCell::new();
    let spi_buffer = SPI_BUF.init([0u8; 512]);
    let di = SpiInterface::new(spi_device, dc, spi_buffer);

    // Initialize display
    let mut display = Builder::new(ST7789, di)
        .reset_pin(rst)
        .display_size(135, 240)
        .display_offset(52, 40)
        .color_order(ColorOrder::Rgb)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut Delay)
        .unwrap();

    log::info!("Display initialized!");

    // Import graphics libraries
    use embedded_graphics::{
        mono_font::{MonoTextStyle, ascii::FONT_9X15_BOLD},
        prelude::*,
        text::Text,
    };
    use embedded_graphics_core::pixelcolor::{Rgb565, RgbColor};

    // Setup WiFi
    let rng = Rng::new();
    let (controller, stack, runner) = wifi::setup_wifi(peripherals.WIFI, rng);

    // Spawn WiFi tasks
    wifi::spawn_tasks(&spawner, controller, stack, runner).await;

    log::info!("WiFi tasks spawned!");

    // Get time from NTP server
    let base_ntp_time = match ntp::get_ntp_time(&stack).await {
        Ok(time) => {
            log::info!(
                "NTP time synchronized: {:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                time.year,
                time.month,
                time.day,
                time.hour,
                time.minute,
                time.second
            );
            time
        }
        Err(e) => {
            log::info!("Failed to get NTP time: {}", e);
            // Use default time if NTP fails
            ntp::NtpTime {
                year: 2024,
                month: 1,
                day: 1,
                hour: 8,
                minute: 0,
                second: 0,
            }
        }
    };

    // Record the reference time
    let start_instant = embassy_time::Instant::now();

    // Main loop: Update timetable periodically
    loop {
        // Calculate elapsed time
        let elapsed = start_instant.elapsed();
        let elapsed_seconds = elapsed.as_secs();

        // Calculate current time (NTP base time + elapsed time)
        let current_ntp_time = base_ntp_time.add_seconds(elapsed_seconds);
        let current_time = Time::new(current_ntp_time.hour, current_ntp_time.minute);

        // Get next 3 trains
        let next_trains = get_next_trains(current_time, 3);

        log::info!(
            "Current time: {:02}:{:02}",
            current_time.hour,
            current_time.minute
        );
        // Log next trains individually
        for (i, train) in next_trains.iter().enumerate() {
            log::info!(
                "Next train {}: {:02}:{:02}",
                i + 1,
                train.hour,
                train.minute
            );
        }

        display.clear(Rgb565::BLACK).ok();

        // Display title
        let title_style = MonoTextStyle::new(&FONT_9X15_BOLD, Rgb565::CYAN);
        Text::new("Sobu Line Rapid", Point::new(5, 15), title_style)
            .draw(&mut display)
            .ok();
        Text::new("Shin-Koiwa Sta.", Point::new(5, 30), title_style)
            .draw(&mut display)
            .ok();

        // Display current time
        let time_style = MonoTextStyle::new(&FONT_9X15_BOLD, Rgb565::YELLOW);
        let mut time_str = heapless::String::<16>::new();
        let _ = core::fmt::write(
            &mut time_str,
            format_args!("Now: {:02}:{:02}", current_time.hour, current_time.minute),
        );
        Text::new(&time_str, Point::new(5, 55), time_style)
            .draw(&mut display)
            .ok();

        // Display next trains
        let train_style = MonoTextStyle::new(&FONT_9X15_BOLD, Rgb565::WHITE);
        let next_label_style = MonoTextStyle::new(&FONT_9X15_BOLD, Rgb565::GREEN);

        Text::new("Next trains:", Point::new(5, 80), next_label_style)
            .draw(&mut display)
            .ok();

        for (i, train) in next_trains.iter().enumerate() {
            let y_pos = 100 + (i as i32 * 20);
            let mut train_str = heapless::String::<16>::new();
            let _ = core::fmt::write(
                &mut train_str,
                format_args!("  {:02}:{:02}", train.hour, train.minute),
            );
            Text::new(&train_str, Point::new(5, y_pos), train_style)
                .draw(&mut display)
                .ok();
        }

        // Update every 5 seconds
        Timer::after(Duration::from_secs(5)).await;
    }
}
