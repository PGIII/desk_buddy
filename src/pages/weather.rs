use anyhow::anyhow;
use embedded_graphics::{
    image::Image,
    mono_font::MonoTextStyle,
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, StrokeAlignment},
    text::{renderer::TextRenderer, Text},
};
use profont::{PROFONT_14_POINT, PROFONT_24_POINT};
use ssd1680::driver::DisplayError;
use thiserror::Error;
use tinybmp::Bmp;

use crate::{openweather::api::get_forecast, ConfigWeather, IconsOW40};

#[derive(Debug, Error)]
pub enum WeatherPageError {
    #[error("Display Error")]
    Display(DisplayError),
    #[error("BMP")]
    BMP(tinybmp::ParseError),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

pub fn draw<D>(display: &mut D, config: &ConfigWeather) -> Result<(), WeatherPageError>
where
    D: DrawTarget<Color = BinaryColor, Error = DisplayError>,
{
    let weather = get_forecast(config)?;
    let Size { width, height } = display.bounding_box().size;
    let width = width as i32;
    let height = height as i32;
    let icon_path = format!("{}.bmp", &weather.list[0].weather[0].icon);
    let Some(icon) = IconsOW40::get(&icon_path) else {
        return Err(anyhow!("Couldn't find icon: {}", icon_path).into());
    };
    let bmp = Bmp::<BinaryColor>::from_slice(&icon.data).map_err(|e| WeatherPageError::BMP(e))?;
    let large_text = MonoTextStyle::new(&PROFONT_24_POINT, BinaryColor::Off);
    let small_text = MonoTextStyle::new(&PROFONT_14_POINT, BinaryColor::Off);
    let temp_str = format!("{:.0}F", weather.list[0].main.temp);
    let weather_str = format!(
        "Forecast For {}\nH{:.0}F L{:.0}F Now{:.0}F\n{}",
        weather.city.name,
        weather.list[0].main.temp_max,
        weather.list[0].main.temp_min,
        weather.list[0].main.temp,
        weather.list[0].weather[0].main
    );
    // background fill
    display
        .fill_solid(&display.bounding_box(), BinaryColor::On)
        .map_err(|e| WeatherPageError::Display(e))?;
    Text::with_baseline(
        &weather.city.name,
        Point::new(0, 0),
        small_text,
        embedded_graphics::text::Baseline::Top,
    )
    .draw(display)
    .map_err(|e| WeatherPageError::Display(e))?;
    Text::with_baseline(
        &temp_str,
        Point::new(0, small_text.line_height() as i32),
        large_text,
        embedded_graphics::text::Baseline::Top,
    )
    .draw(display)
    .map_err(|e| WeatherPageError::Display(e))?;
    Image::new(&bmp, Point::new(width - 40, 0))
        .draw(display)
        .map_err(|e| WeatherPageError::Display(e))?;
    Ok(())
}
