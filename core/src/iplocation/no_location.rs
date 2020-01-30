use super::{IpLocation, IpLocationError, IpLocationProvider};
use std::net::IpAddr;

/// Ip location provider that always fails
#[derive(Clone)]
pub struct IpNoLocation;

impl IpLocationProvider for IpNoLocation {
    fn get_location(&self, _ip: IpAddr) -> Result<IpLocation, IpLocationError> {
        Err(IpLocationError::LocationUnknown)
    }
}
