pub mod pages;

use db_weather_openweather::OpenWeather;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver};
use esp_idf_svc::hal::modem::WifiModemPeripheral;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver, SpiDriverConfig};
use esp_idf_svc::sys::EspError;
use esp_idf_svc::{hal::delay::Ets, log::EspLogger};
use log::info;
use rust_embed::RustEmbed;
use serde::Deserialize;
use ssd1680::prelude::*;
use std::thread::sleep;
use std::time::Duration;

#[derive(RustEmbed)]
#[folder = "images/bmp/40/"]
struct Icons40;

#[derive(RustEmbed)]
#[folder = "images/bmp/20/"]
struct Icons20;

#[derive(Debug, Deserialize)]
pub struct Config {
    wifi: ConfigWifi,
    weather: ConfigWeather,
}

#[derive(Debug, Deserialize)]
pub struct ConfigWifi {
    ssid: String,
    password: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigWeather {
    api_key: String,
    zip_code: u32,
    country_code: String,
}

// Don't forget to raise the CONFIG_PTHREAD_TASK_STACK_SIZE_DEFAULT in `sdkconfig.defaults` to > 4K so that the
// `async-io` background thread can work fine
pub fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    EspLogger::initialize_default();

    //Load Config
    info!("loading config");
    let config_str = include_str!("../config.toml");
    let config: Config = toml::from_str(config_str)?;
    let weather_api = OpenWeather::new(
        &config.weather.api_key,
        config.weather.zip_code,
        &config.weather.country_code,
    );
    let peripherals = Peripherals::take()?;
    let _wifi = wifi_create(peripherals.modem, &config.wifi)?;

    let spi = peripherals.spi2;

    let rst = PinDriver::output(peripherals.pins.gpio13)?;
    let dc = PinDriver::output(peripherals.pins.gpio12)?;
    let busy = PinDriver::input(peripherals.pins.gpio14)?;
    let mut delay = Ets;

    let sclk = peripherals.pins.gpio10;
    let sdo = peripherals.pins.gpio9;

    let spi = SpiDriver::new(
        spi,
        sclk,
        sdo,
        None::<AnyIOPin>,
        &SpiDriverConfig::default(),
    )?;

    let cs = peripherals.pins.gpio11;

    let spi = SpiDeviceDriver::new(spi, Some(cs), &esp_idf_svc::hal::spi::config::Config::new())?;

    let mut ssd1680 = Ssd1680::new(spi, busy, dc, rst, &mut delay).unwrap();
    ssd1680.clear_bw_frame().unwrap();
    let mut display_bw = Display2in13::bw();
    display_bw.set_rotation(DisplayRotation::Rotate90);

    loop {
        pages::weather::draw(&mut display_bw, &weather_api)?;
        ssd1680.update_bw_frame(display_bw.buffer()).unwrap();
        ssd1680.display_frame(&mut FreeRtos).unwrap();
        sleep(Duration::from_secs(60 * 10)); //10min
    }
}

fn wifi_create(
    modem: impl WifiModemPeripheral + 'static,
    config: &ConfigWifi,
) -> Result<esp_idf_svc::wifi::EspWifi<'static>, EspError> {
    use esp_idf_svc::eventloop::*;
    use esp_idf_svc::nvs::*;
    use esp_idf_svc::wifi::*;

    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut esp_wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs.clone()))?;
    let mut wifi = BlockingWifi::wrap(&mut esp_wifi, sys_loop.clone())?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: config.ssid.as_str().try_into().unwrap(),
        password: config.password.as_str().try_into().unwrap(),
        ..Default::default()
    }))?;

    wifi.start()?;
    info!("Wifi started");

    wifi.connect()?;
    info!("Wifi connected");

    wifi.wait_netif_up()?;
    info!("Wifi netif up");

    Ok(esp_wifi)
}
