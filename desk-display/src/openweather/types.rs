use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Forecast {
    pub cod: String,
    pub message: i64,
    pub cnt: i64,
    pub list: Vec<List>,
    pub city: City,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct List {
    pub dt: i64,
    pub main: Main,
    pub weather: Vec<Weather>,
    pub clouds: Clouds,
    pub wind: Wind,
    pub visibility: Option<i64>,
    pub pop: f64,
    pub rain: Option<Rain>,
    pub sys: Sys,
    pub dt_txt: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Main {
    pub temp: f64,
    pub feels_like: f64,
    pub temp_min: f64,
    pub temp_max: f64,
    pub pressure: i64,
    pub sea_level: i64,
    pub grnd_level: i64,
    pub humidity: i64,
    pub temp_kf: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Weather {
    pub id: i64,
    pub main: String,
    pub description: String,
    pub icon: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clouds {
    pub all: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wind {
    pub speed: f64,
    pub deg: i64,
    pub gust: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rain {
    #[serde(rename = "3h")]
    pub n3h: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sys {
    pub pod: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct City {
    pub id: i64,
    pub name: String,
    pub coord: Coord,
    pub country: String,
    pub population: i64,
    pub timezone: i64,
    pub sunrise: i64,
    pub sunset: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoLocationZip {
    pub zip: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub country: String,
}
