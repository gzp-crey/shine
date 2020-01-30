use super::{IpLocation, IpLocationError, IpLocationProvider};
use std::collections::HashMap;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;

pub struct IpDataLocationConfig {
    api_key: String,
}

/// Ip location provider using https://ipdata.co
#[derive(Clone)]
pub struct IpDataLocation {
    api_key: String,
}

impl IpDataLocation {
    pub fn new(config: IpDataLocationConfig) -> IpDataLocation {
        IpDataLocation { api_key: config.api_key }
    }

    async fn location_request(&self, ip: IpAddr) -> Result<IpLocation, IpLocationError> {
        let uri = format!("https://api.ipdata.co/apikey={}", self.api_key);
        let raw = reqwest::get(&uri)
            .await
            .map_err(|err| IpLocationError::ExternalProvider(err.to_string()))?
            .text()
            .await
            .map_err(|err| IpLocationError::ExternalProvider(err.to_string()))?;
        log::debug!("raw: {:?}", raw);
        let values: HashMap<String, String> = serde_json::from_slice(&raw).map_err(crate::error::decode)?;
        log::debug!("values: {:?}", values);
        unimplemented!()
    }
}

impl IpLocationProvider for IpDataLocation {
    fn get_location<'s>(&'s self, ip: IpAddr) -> Pin<Box<dyn Future<Output = Result<IpLocation, IpLocationError>> + 's>> {
        Box::pin(self.get_location(ip))
    }
}
