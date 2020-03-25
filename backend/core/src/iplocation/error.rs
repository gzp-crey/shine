use azure_sdk_core::errors::AzureError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum IpLocationError {
    /// Database related error
    Internal(String),

    /// Provider service error
    External(String),

    /// The ip address is private (ex localhost)
    Private,

    /// Could not determine ip location
    Unknown,
}

impl fmt::Display for IpLocationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IpLocationError::Internal(ref e) => write!(f, "DB, {}", e),
            IpLocationError::External(ref e) => write!(f, "Ip location query failed from external provider: {}", e),
            IpLocationError::Unknown => write!(f, "Ip location could not be determined"),
            IpLocationError::Private => write!(f, "Ip location is private"),
        }
    }
}

impl Error for IpLocationError {}

impl From<AzureError> for IpLocationError {
    fn from(err: AzureError) -> IpLocationError {
        IpLocationError::Internal(format!("Azure db error: {:?}", err))
    }
}
