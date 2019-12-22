use crate::backoff::BackoffError;
use azure_sdk_core::errors::AzureError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum IdSequenceError {
    /// Database related error
    DB(String),
    /// Sequence is out of id
    SequenceEnded,
}

impl fmt::Display for IdSequenceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IdSequenceError::DB(ref e) => write!(f, "DB, {}", e),
            IdSequenceError::SequenceEnded => write!(f, "Sequence is out of id"),
        }
    }
}

impl Error for IdSequenceError {}

impl From<AzureError> for IdSequenceError {
    fn from(err: AzureError) -> IdSequenceError {
        IdSequenceError::DB(format!("{:?}", err))
    }
}

impl From<BackoffError<AzureError>> for IdSequenceError {
    fn from(err: BackoffError<AzureError>) -> IdSequenceError {
        match err {
            BackoffError::RetryLimit(r, t) => IdSequenceError::DB(format!("Retry limit reached, retry:{}, timeout:{}", r, t)),
            BackoffError::Action(e) => IdSequenceError::DB(format!("{:?}", e)),
            BackoffError::Retry { .. } => IdSequenceError::DB(format!("Internal error")),
        }
    }
}

#[cfg(feature = "actix_web")]
mod feature_actix_web {
    use actix_web::ResponseError;
    impl ResponseError for IdSequenceError {
        // Default to 500 for now
    }
}
