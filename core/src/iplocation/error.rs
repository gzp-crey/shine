use azure_sdk_core::errors::AzureError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum IpLocationError {
    /// Database related error
    DB(String),

    /// Provider service error
    ExternalProvider(String),

    /// Could not determine ip location
    LocationUnknown,
}

impl fmt::Display for IpLocationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IpLocationError::DB(ref e) => write!(f, "DB, {}", e),
            IpLocationError::LocationUnknown => write!(f, "Ip location is not known"),
            IpLocationError::ExternalProvider(ref e) => write!(f, "Ip location query failed from external provider: {}", e),
        }
    }
}

impl Error for IpLocationError {}

impl From<AzureError> for IpLocationError {
    fn from(err: AzureError) -> IpLocationError {
        IpLocationError::DB(format!("{:?}", err))
    }
}
