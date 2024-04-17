use anyhow::anyhow;
use chrono::{DateTime, FixedOffset};
use db_weather::Condition;
use memchr::memmem;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use url::{Position, Url};

use super::types::{self, Forecast, GeoLocationZip};

impl From<Forecast> for db_weather::Forecast {
    fn from(value: Forecast) -> Self {
        let today = &value.list[0];
        let dt = DateTime::from_timestamp(today.dt, 0).unwrap();
        let tz = FixedOffset::east_opt(value.city.timezone.try_into().unwrap()).unwrap();
        let dt = dt.with_timezone(&tz);
        Self {
            city: value.city.name,
            temperature: today.main.temp,
            temperature_max: today.main.temp_max,
            temperature_min: today.main.temp_min,
            feels_like: today.main.feels_like,
            time_period: Duration::from_secs(60 * 60 * 24),
            date_time: dt,
            condition: Condition::from(today.weather[0].clone()),
            children: vec![],
        }
    }
}

impl From<types::Weather> for Condition {
    fn from(value: types::Weather) -> Self {
        match value.main.as_str() {
            "Rain" => Condition::Rain(0),
            "Clouds" => Condition::Cloudy,
            "Mist" => Condition::Mist,
            _ => Condition::Clear,
        }
    }
}
