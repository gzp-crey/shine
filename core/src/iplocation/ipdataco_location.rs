use super::{IpLocation, IpLocationError, IpLocationProvider};
use serde::Deserialize;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;

pub struct IpLocationIpDataCoConfig {
    pub api_key: String,
}

#[derive(Debug, Deserialize)]
struct IpDataCoResponse {
    country_code: String,
    continent_code: String,
    longitude: f32,
    latitude: f32,
}

/// Ip location provider using https://ipdata.co
#[derive(Clone)]
pub struct IpLocationIpDataCo {
    api_key: String,
}

impl IpLocationIpDataCo {
    pub fn new(config: IpLocationIpDataCoConfig) -> IpLocationIpDataCo {
        IpLocationIpDataCo { api_key: config.api_key }
    }

    async fn location_request(&self, ip: &IpAddr) -> Result<IpLocation, IpLocationError> {
        let uri = format!("https://api.ipdata.co/{}?api-key={}", ip.to_string(), self.api_key);
        let raw = reqwest::get(&uri)
            .await
            .map_err(|err| IpLocationError::External(err.to_string()))?
            .text()
            .await
            .map_err(|err| IpLocationError::External(err.to_string()))?;
        let response: IpDataCoResponse = serde_json::from_slice(raw.as_bytes())
            .map_err(|_| IpLocationError::External(format!("Failed to parse response: {}", raw)))?;

        Ok(IpLocation {
            country: response.country_code,
            continent: response.continent_code,
            extended: Some(raw),
        })
    }
}

impl IpLocationProvider for IpLocationIpDataCo {
    fn get_location<'s>(&'s self, ip: &'s IpAddr) -> Pin<Box<dyn Future<Output = Result<IpLocation, IpLocationError>> + 's>> {
        Box::pin(self.location_request(&ip))
    }
}
