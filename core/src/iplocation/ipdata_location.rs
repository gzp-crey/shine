use super::{IpLocation, IpLocationError, IpLocationProvider};
use std::net::IpAddr;

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
}

impl IpLocationProvider for IpDataLocation {
    fn get_location(&self, ip: IpAddr) -> Result<IpLocation, IpLocationError> {
        unimplemented!()
    }
}
