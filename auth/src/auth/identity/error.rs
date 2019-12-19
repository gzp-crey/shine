use actix_web::ResponseError;
use azure_sdk_core::errors::AzureError;
use std::fmt;

#[derive(Debug)]
pub enum IdentityError {
    /// Database related error
    DB(String),
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IdentityError::DB(ref e) => write!(f, "DB, {}", e),
        }
    }
}

impl ResponseError for IdentityError {
    // Default to 500 for now
}

impl From<AzureError> for IdentityError {
    fn from(err: AzureError) -> IdentityError {
        IdentityError::DB(format!("{:?}", err))
    }
}
