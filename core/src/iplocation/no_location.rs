use super::{IpLocation, IpLocationError, IpLocationProvider};
use futures::future::ready;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;

/// Ip location provider that always fails
#[derive(Clone)]
pub struct IpNoLocation;

impl IpLocationProvider for IpNoLocation {
    fn get_location<'s>(&'s self, ip: IpAddr) -> Pin<Box<dyn Future<Output = Result<IpLocation, IpLocationError>> + 's>> {
        Box::pin(ready(Err(IpLocationError::LocationUnknown)))
    }
}
