#![no_std]
#![no_main]
#![deny(clippy::large_stack_frames)]

use esp_backtrace as _;
use esp_hal::{main, rng::Rng, timer::timg::TimerGroup};
use esp_radio::wifi::{
    AccessPointConfig, AuthMethod, ClientConfig, Config, ModeConfig, ScanConfig, WifiController,
};
use log::info;
use smoltcp::iface::{SocketSet, SocketStorage};

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const SSID: &str = "iPhone-JLKQ661CQW";
const PASSWORD: &str = "motopass";

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
// #[main]
// fn main() -> ! {
#[esp_rtos::main]
async fn main(spawner: embassy_executor::Spawner) -> ! {
    // Use default CPU clock to avoid UART baud rate issues
    let config = esp_hal::Config::default();
    // Start the system
    let peripherals = esp_hal::init(config);

    esp_println::logger::init_logger_from_env();

    esp_alloc::heap_allocator!(size: 128000);

    info!("System initialized!");

    // initialize wifi controller
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let _rng = Rng::new();
    esp_rtos::start(timg0.timer0);
    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");

    let wifi_config = Config::default();
    let (mut wifi_controller, interfaces) =
        esp_radio::wifi::new(&radio_init, peripherals.WIFI, wifi_config)
            .expect("Failed to initialize Wi-Fi controller");

    let _device = interfaces.sta;

    let delay = esp_hal::delay::Delay::new();

    configure_wifi(&mut wifi_controller).await;
    // scan_wifi(&mut wifi_controller);
    connect_wifi(&mut wifi_controller).await;

    loop {
        info!("Hello from M5Stack C!");
        delay.delay_millis(1000);
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0/examples
}

async fn configure_wifi(controller: &mut WifiController<'_>) {
    controller
        .set_power_saving(esp_radio::wifi::PowerSaveMode::Maximum)
        .unwrap();

    let ap_config = ModeConfig::Client(
        ClientConfig::default()
            .with_ssid(SSID.into())
            .with_password(PASSWORD.into())
            .with_auth_method(esp_radio::wifi::AuthMethod::Wpa2Personal),
    );
    let res = controller.set_config(&ap_config).unwrap();
    info!("wifi_set_configuration returned {:?}", res);

    controller.start_async().await.unwrap();
    while !controller.is_started().unwrap() {}
}

fn scan_wifi(controller: &mut WifiController<'_>) {
    info!("Start Wifi Scan");

    let res = controller.scan_with_config(ScanConfig::default()).unwrap();

    let mut found = false;

    for ap in res {
        info!("ap: {:?}", ap);

        if ap.ssid.as_str() == SSID {
            found = true;
            info!("FOUND TARGET SSID: {:?}", ap);
        }
    }

    info!("target ssid found: {}", found);
}

async fn connect_wifi(controller: &mut WifiController<'_>) {
    info!("{:?}", controller.capabilities());

    match controller.connect() {
        Ok(()) => info!("connect_async returned Ok"),
        Err(e) => {
            info!("connect_async error: {:?}", e);
            return;
        }
    }

    info!("Wait to get connected");

    loop {
        match controller.is_connected() {
            Ok(true) => break,
            Ok(false) => {
                info!("not connected yet");
                esp_rtos::CurrentThreadHandle::get()
                    .delay(esp_hal::time::Duration::from_millis(2000));
            }
            Err(err) => {
                info!("is_connected error: {err:?}");
                esp_rtos::CurrentThreadHandle::get()
                    .delay(esp_hal::time::Duration::from_millis(1000));

                controller.disconnect().unwrap();

                esp_rtos::CurrentThreadHandle::get()
                    .delay(esp_hal::time::Duration::from_millis(1000));

                controller.stop().unwrap();

                esp_rtos::CurrentThreadHandle::get()
                    .delay(esp_hal::time::Duration::from_millis(1000));

                controller.start().unwrap();

                info!("restart done");

                while !controller.is_started().unwrap() {}

                match controller.connect() {
                    Ok(()) => info!("re-connect_async returned Ok"),
                    Err(e) => {
                        info!("connect_async error: {:?}", e);
                        return;
                    }
                }
            }
        }
    }

    info!("Connected");
}
