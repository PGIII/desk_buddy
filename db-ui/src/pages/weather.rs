use std::convert::Infallible;

use crate::Icons40;
use anyhow::anyhow;
use db_weather::Condition;
use db_weather_openweather::OpenWeather;
use display_interface::DisplayError;
use embedded_graphics::{
    image::Image,
    mono_font::MonoTextStyle,
    pixelcolor::BinaryColor,
    prelude::*,
    text::{renderer::TextRenderer, Text, TextStyleBuilder},
};
use profont::{PROFONT_14_POINT, PROFONT_24_POINT};
use thiserror::Error;
use tinybmp::Bmp;

#[derive(Debug, Error)]
pub enum WeatherPageError {
    #[error("Display Error")]
    Display(DisplayError),
    #[error("BMP")]
    BMP(tinybmp::ParseError),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

impl From<Infallible> for WeatherPageError {
    fn from(_value: Infallible) -> Self {
        unreachable!()
    }
}

pub fn draw<D, E>(display: &mut D, weather_api: &OpenWeather) -> Result<(), WeatherPageError>
where
    E: Into<WeatherPageError>,
    D: DrawTarget<Color = BinaryColor, Error = E>,
{
    let weather = weather_api.get_forecast()?;
    let Size { width, height: _ } = display.bounding_box().size;
    let width = width as i32;
    let icon_path = condition_to_image(weather.condition);
    let Some(icon) = Icons40::get(&icon_path) else {
        return Err(anyhow!("Couldn't find icon: {}", icon_path).into());
    };
    let bmp = Bmp::<BinaryColor>::from_slice(&icon.data).map_err(|e| WeatherPageError::BMP(e))?;
    let large_text = MonoTextStyle::new(&PROFONT_24_POINT, BinaryColor::Off);
    let small_text = MonoTextStyle::new(&PROFONT_14_POINT, BinaryColor::Off);
    let right_align = TextStyleBuilder::new()
        .baseline(embedded_graphics::text::Baseline::Top)
        .alignment(embedded_graphics::text::Alignment::Right)
        .build();
    let now = weather.date_time;
    let city_text = format!("{} {}", weather.city, now.format("%I:%M %p"));

    let temp_str = format!("{:.0}F", weather.temperature);
    let high_low_temp = format!(
        "H{:.0} L{:.0}",
        weather.temperature_max, weather.temperature_min,
    );

    // background fill
    display
        .fill_solid(&display.bounding_box(), BinaryColor::On)
        .map_err(|e| e.into())?;

    Text::with_baseline(
        &city_text,
        Point::new(0, 0),
        small_text,
        embedded_graphics::text::Baseline::Top,
    )
    .draw(display)
    .map_err(|e| e.into())?;

    Text::with_baseline(
        &temp_str,
        Point::new(0, small_text.line_height() as i32),
        large_text,
        embedded_graphics::text::Baseline::Top,
    )
    .draw(display)
    .map_err(|e| e.into())?;

    Text::with_baseline(
        &high_low_temp,
        Point::new(
            0,
            large_text.line_height() as i32 + small_text.line_height() as i32,
        ),
        small_text,
        embedded_graphics::text::Baseline::Top,
    )
    .draw(display)
    .map_err(|e| e.into())?;

    Text::with_text_style(
        (&weather.condition).into(),
        Point::new(width, 40),
        small_text,
        right_align,
    )
    .draw(display)
    .map_err(|e| e.into())?;
    Image::new(&bmp, Point::new(width - 40, 0))
        .draw(display)
        .map_err(|e| e.into())?;
    Ok(())
}

/// Returns bmp file name to load
fn condition_to_image(condition: Condition) -> &'static str {
    match condition {
        Condition::Rain(_) => "18.bmp",
        Condition::Mist => "10.bmp",
        Condition::Clear => "2.bmp",
        Condition::Cloudy => "14.bmp",
        Condition::Wind(_) => "6.bmp",
        Condition::Snow(_) => "7.bmp",
    }
}
