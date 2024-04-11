pub mod openweather;

use std::thread::sleep;
use std::time::Duration;

use anyhow::anyhow;
use embedded_graphics::geometry::Point;
use embedded_graphics::image::Image;
use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Circle, Primitive, PrimitiveStyle, PrimitiveStyleBuilder, StrokeAlignment,
};
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::modem::WifiModemPeripheral;
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver, SpiDriverConfig};
use esp_idf_svc::sys::EspError;
use esp_idf_svc::{hal::delay::Ets, log::EspLogger};
use log::info;
use openweather::api::get_forecast;
use rust_embed::RustEmbed;
use serde::Deserialize;
use thiserror::Error;

use esp_idf_svc::hal::gpio::{AnyIOPin, PinDriver};
use esp_idf_svc::hal::peripherals::Peripherals;
use ssd1680::driver::DisplayError;
use ssd1680::prelude::*;
use tinybmp::Bmp;

#[derive(RustEmbed)]
#[folder = "images/bmp/ow/40/"]
struct Icons40;

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
    let config_str = include_str!("../config.toml");
    let config: Config = toml::from_str(config_str)?;

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
        ssd1680.clear_bw_frame().unwrap();
        draw_weather_page(&mut display_bw, &config.weather)?;
        ssd1680.update_bw_frame(display_bw.buffer()).unwrap();
        ssd1680.display_frame(&mut FreeRtos).unwrap();
        sleep(Duration::from_secs(60 * 10)); //10min
    }
}

#[derive(Debug, Error)]
enum WeatherPageError {
    #[error("Display Error")]
    Display(DisplayError),
    #[error("BMP")]
    BMP(tinybmp::ParseError),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

fn draw_weather_page<D>(display: &mut D, config: &ConfigWeather) -> Result<(), WeatherPageError>
where
    D: DrawTarget<Color = BinaryColor, Error = DisplayError>,
{
    let border_stroke = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::Off)
        .stroke_width(3)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();
    display
        .bounding_box()
        .into_styled(border_stroke)
        .draw(display)
        .map_err(|e| WeatherPageError::Display(e))?;

    let weather = get_forecast(config)?;
    let weather_str = format!(
        "Forecast For {}\nH{:.0}F L{:.0}F Now{:.0}F\n{}",
        weather.city.name,
        weather.list[0].main.temp_max,
        weather.list[0].main.temp_min,
        weather.list[0].main.temp,
        weather.list[0].weather[0].main
    );
    let style = MonoTextStyle::new(&FONT_10X20, BinaryColor::Off);
    let icon_path = format!("{}.bmp", &weather.list[0].weather[0].icon);
    let Some(icon) = Icons40::get(&icon_path) else {
        return Err(anyhow!("Couldn't find icon: {}", icon_path).into());
    };
    let bmp = Bmp::<BinaryColor>::from_slice(&icon.data).map_err(|e| WeatherPageError::BMP(e))?;
    Text::new(&weather_str, Point::new(52, 20), style)
        .draw(display)
        .map_err(|e| WeatherPageError::Display(e))?;
    Image::new(&bmp, Point::new(10, 2))
        .draw(display)
        .map_err(|e| WeatherPageError::Display(e))?;
    Ok(())
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
