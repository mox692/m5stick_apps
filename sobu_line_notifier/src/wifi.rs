use embassy_executor::Spawner;
use embassy_net::{Config, Runner, Stack, StackResources};
use embassy_time::Timer;
use esp_radio::wifi::{
    ClientConfig, ModeConfig, WifiController, WifiDevice, WifiStaState, sta_state,
};
use static_cell::StaticCell;

const STABLE_CONNECTION_RECHECK_DELAY_MS: u64 = 5000;
const UNSTABLE_CONNECTION_RECHECK_DELAY_MS: u64 = 1000;

static RADIO_INIT: StaticCell<esp_radio::Controller> = StaticCell::new();
static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();

#[embassy_executor::task]
async fn connection_task(mut controller: WifiController<'static>) {
    let (ssid, password) = load_wifi_credentials();

    let config = ModeConfig::Client(
        ClientConfig::default()
            .with_ssid(ssid.into())
            .with_password(password.into()),
    );

    controller
        .set_config(&config)
        .expect("Failed to set Wi-Fi config");

    log::info!("Connecting to SSID: {}", ssid);

    controller
        .start_async()
        .await
        .expect("Failed to start Wi-Fi connection");

    loop {
        match sta_state() {
            WifiStaState::Started => {
                if let Err(e) = controller.connect_async().await {
                    log::error!("Failed to connect to Wi-Fi: {:?}", e);
                } else {
                    log::info!("Connected to Wifi");
                }
            }
            WifiStaState::Connected => {
                Timer::after_millis(STABLE_CONNECTION_RECHECK_DELAY_MS).await;
                continue;
            }
            WifiStaState::Disconnected => {
                log::warn!("Wi-Fi disconnected, retry connection");
                if let Err(e) = controller.connect_async().await {
                    log::error!("Failed to reconnect to Wi-Fi: {:?}", e);
                } else {
                    log::info!("Reconnected to Wifi");
                }
            }
            WifiStaState::Stopped => {
                log::warn!("Wi-Fi stopped, restarting connection");
                if let Err(e) = controller.start_async().await {
                    log::error!("Failed to restart Wi-Fi connection: {:?}", e);
                };
            }
            WifiStaState::Invalid => {
                log::error!("Invalid Wi-Fi state");
                if let Err(e) = controller.stop_async().await {
                    log::error!("Failed to stop Wi-Fi connection: {:?}", e);
                }
            }
            _ => {
                log::warn!("Unexpected Wi-Fi state: {:?}", sta_state());
            }
        }

        Timer::after_millis(UNSTABLE_CONNECTION_RECHECK_DELAY_MS).await;
    }
}

#[embassy_executor::task]
async fn network_monitor_task(stack: Stack<'static>) {
    loop {
        if stack.is_link_up()
            && let Some(config) = stack.config_v4()
        {
            log::info!("Network is up with IP: {}", config.address);
        }
        Timer::after_millis(5000).await;
    }
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await
}

pub async fn spawn_tasks(
    spawner: &Spawner,
    controller: WifiController<'static>,
    stack: Stack<'static>,
    runner: Runner<'static, WifiDevice<'static>>,
) {
    spawner.spawn(connection_task(controller)).unwrap();
    spawner.spawn(network_monitor_task(stack)).unwrap();
    spawner.spawn(net_task(runner)).unwrap();
}

pub fn setup_wifi(
    peripheral: esp_hal::peripherals::WIFI<'static>,
    rng: esp_hal::rng::Rng,
) -> (
    WifiController<'static>,
    Stack<'static>,
    Runner<'static, WifiDevice<'static>>,
) {
    let radio_init = esp_radio::init().expect("Failed to initialize Wi-Fi/BLE controller");
    let radio_init = RADIO_INIT.init(radio_init);
    let (controller, interfaces) = esp_radio::wifi::new(radio_init, peripheral, Default::default())
        .expect("Failed to initialize Wi-Fi controller");
    let device = interfaces.sta;

    let config = Config::dhcpv4(Default::default());
    let seed = rng.random() as u64 | ((rng.random() as u64) << 32);

    let (stack, runner) =
        embassy_net::new(device, config, RESOURCES.init(StackResources::new()), seed);

    (controller, stack, runner)
}

fn load_wifi_credentials() -> (&'static str, &'static str) {
    let mut ssid = "";
    let mut password = "";

    let wifi_config = include_str!("../.wifi.config");
    for line in wifi_config.lines() {
        if let Some(stripped) = line.strip_prefix("SSID=") {
            ssid = stripped.trim_matches('"');
        } else if let Some(stripped) = line.strip_prefix("PASSWORD=") {
            password = stripped.trim_matches('"');
        }
    }
    if ssid.is_empty() || password.is_empty() {
        panic!("SSID or PASSWORD not found in .wifi.config");
    }

    (ssid, password)
}
