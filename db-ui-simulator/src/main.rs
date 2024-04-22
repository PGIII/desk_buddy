use db_weather_openweather::OpenWeather;
use embedded_graphics::{geometry::Size, pixelcolor::BinaryColor};
use embedded_graphics_simulator::{
    BinaryColorTheme, OutputSettings, OutputSettingsBuilder, SimulatorDisplay, Window,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    api_key: String,
    zip_code: u32,
    country_code: String,
}

fn main() -> Result<(), core::convert::Infallible> {
    let mut display = SimulatorDisplay::<BinaryColor>::new(Size::new(296, 176));

    let config_str = include_str!("../config.toml");
    let config: Config = toml::from_str(config_str).unwrap();
    let weather_api = OpenWeather::new(&config.api_key, config.zip_code, &config.country_code);
    db_ui::pages::weather::draw(&mut display, &weather_api).unwrap();

    let output_settings = OutputSettingsBuilder::new()
        .theme(BinaryColorTheme::Default)
        .build();
    Window::new("Desk Display", &output_settings).show_static(&display);
    Ok(())
}
