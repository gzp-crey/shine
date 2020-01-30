use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;

mod cached_location;
mod error;
mod ipdata_location;
mod no_location;

pub use self::cached_location::*;
pub use self::error::*;
pub use self::ipdata_location::*;
pub use self::no_location::*;

/// Geo-location of an ip addres
pub struct IpLocation {
    pub country: String,
    pub continent: String,

    /// extended location info provided by some implementations
    pub extended: Option<String>,
}

/// Trait to query geo-location by ip addresses
pub trait IpLocationProvider {
    fn get_location<'s>(&'s self, ip: IpAddr) -> Pin<Box<dyn Future<Output = Result<IpLocation, IpLocationError>> + 's>>;
}
