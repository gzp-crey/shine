use actix_web::http::StatusCode;
use actix_web::ResponseError;
use azure_sdk_core::errors::AzureError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum IdSequenceError {
    /// Database related error
    DB(String),
    /// Sequence is out of id
    SequenceEnded,
    /// Failed to generate id as some conflicts could not be resolved
    Conflit,
}

impl fmt::Display for IdSequenceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IdSequenceError::DB(ref e) => write!(f, "DB, {}", e),
            IdSequenceError::SequenceEnded => write!(f, "Sequence is out of id"),
            IdSequenceError::Conflit => write!(f, "Could not generate id due to DB conflicts"),
        }
    }
}

impl Error for IdSequenceError {}

impl From<AzureError> for IdSequenceError {
    fn from(err: AzureError) -> IdSequenceError {
        IdSequenceError::DB(format!("{:?}", err))
    }
}

impl ResponseError for IdSequenceError {
    fn status_code(&self) -> StatusCode {
        match self {
            IdSequenceError::DB(_) | IdSequenceError::SequenceEnded => StatusCode::INTERNAL_SERVER_ERROR,
            IdSequenceError::Conflit => StatusCode::TOO_MANY_REQUESTS,
        }
    }
}
