mod api;
mod types;

use anyhow::anyhow;


use memchr::memmem;
use std::io::{Read, Write};
use std::net::TcpStream;

use types::{Forecast, GeoLocationZip};
use url::{Position, Url};

pub struct OpenWeather {
    api_key: String,
    zip_code: u32,
    country_code: String,
}

impl OpenWeather {
    pub fn new(api_key: &str, zip_code: u32, country_code: &str) -> Self {
        Self {
            api_key: api_key.to_owned(),
            zip_code,
            country_code: country_code.to_owned(),
        }
    }

    pub fn get_forecast(&self) -> anyhow::Result<db_weather::Forecast> {
        Ok(self.get_ow_forecast()?.into())
    }

    pub fn get_ow_forecast(&self) -> anyhow::Result<Forecast> {
        //first get location
        let location = self.get_location()?;
        let url = format!(
        "http://api.openweathermap.org:80/data/2.5/forecast?lat={}&lon={}&appid={}&units=imperial",
        location.lat, location.lon, self.api_key
    );
        let res = get(&url)?;
        //we need to get out the body of the response, so past /r/n/r/n
        let sep_bytes = b"\r\n\r\n";
        let Some(sep_pos) = memmem::find(&res, sep_bytes) else {
            return Err(anyhow!("Missing header body seperator"));
        };
        let body = &res[sep_pos + sep_bytes.len()..];
        Ok(serde_json::from_slice(body)?)
    }

    pub fn get_location(&self) -> anyhow::Result<GeoLocationZip> {
        let url = format!(
            "http://api.openweathermap.org:80/geo/1.0/zip?zip={},{}&appid={}",
            self.zip_code, self.country_code, self.api_key
        );
        let res = get(&url)?;
        //we need to get out the body of the response, so past /r/n/r/n
        let sep_bytes = b"\r\n\r\n";
        let Some(sep_pos) = memmem::find(&res, sep_bytes) else {
            return Err(anyhow!("Missing header body seperator"));
        };
        let body = &res[sep_pos + sep_bytes.len()..];
        Ok(serde_json::from_slice(body)?)
    }
}
fn get(url_into: &str) -> anyhow::Result<Vec<u8>> {
    let url = Url::parse(url_into)?;
    if url.scheme() != "http" {
        Err(anyhow!("Only http is allowed"))
    } else {
        let port = if let Some(url_port) = url.port() {
            url_port
        } else {
            80
        };
        let Some(hostname) = url.host() else {
            return Err(anyhow!("Missing hostname"));
        };
        let path = &url[Position::BeforePath..];

        let mut stream = TcpStream::connect(format!("{hostname}:{port}"))?;
        stream.write_all(format!("GET {path} HTTP/1.0\r\n\r\n").as_bytes())?;
        let mut buf = vec![];
        stream.read_to_end(&mut buf)?;
        Ok(buf)
    }
}
