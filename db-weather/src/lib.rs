use chrono::{FixedOffset};
use core::time::Duration;

#[derive(Debug, Clone, Copy)]
pub enum Condition {
    Rain(u32), //contains rain percent
    Wind(f64), //contains wind speed
    Snow(u32), // contains snow percent
    Cloudy,
    Mist,
    Clear,
}

impl From<&Condition> for &'static str {
    fn from(value: &Condition) -> Self {
        match value {
            Condition::Rain(_) => "Rain",
            Condition::Wind(_) => "Wind",
            Condition::Snow(_) => "Snow",
            Condition::Cloudy => "Cloudy",
            Condition::Clear => "Clear",
            Condition::Mist => "Mist",
        }
    }
}

// Represents forecast for a time period
// could be a day, or hour
pub struct Forecast {
    pub temperature: f64,
    pub temperature_max: f64,
    pub temperature_min: f64,
    pub feels_like: f64,
    pub condition: Condition,
    pub city: String,
    pub time_period: Duration, //Period of time this encompasses
    pub date_time: chrono::DateTime<FixedOffset>,
    pub children: Vec<Self>, // forecast for smaller periods of time within this period
}

#[cfg(test)]
mod tests {}
