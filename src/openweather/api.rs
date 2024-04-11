use crate::ConfigWeather;
use anyhow::anyhow;
use memchr::memmem;
use std::io::{Read, Write};
use std::net::TcpStream;
use url::{Position, Url};

use super::types::{Forecast, GeoLocationZip};

pub fn get_forecast(config: &ConfigWeather) -> anyhow::Result<Forecast> {
    //first get location
    let location = get_location(config)?;
    let url = format!(
        "http://api.openweathermap.org:80/data/2.5/forecast?lat={}&lon={}&appid={}&units=imperial",
        location.lat, location.lon, config.api_key
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

pub fn get_location(config: &ConfigWeather) -> anyhow::Result<GeoLocationZip> {
    let url = format!(
        "http://api.openweathermap.org:80/geo/1.0/zip?zip={},{}&appid={}",
        config.zip_code, config.country_code, config.api_key
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
