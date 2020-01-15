use actix_web::ResponseError;
use serde_json::error::Error as JsonError;
use std::fmt;

/// Errors that can occur during handling signed cookie session
#[derive(Debug)]
pub enum SignedCookieError {
    /// Failed to serialize session.
    Serialize(JsonError),

    /// Signature verification failed
    Verification,
}

impl fmt::Display for SignedCookieError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SignedCookieError::Serialize(err) => write!(f, "Serialization error: {}", err),
            SignedCookieError::Verification => write!(f, "Signature verification failed"),
        }
    }
}

impl From<JsonError> for SignedCookieError {
    fn from(err: JsonError) -> SignedCookieError {
        SignedCookieError::Serialize(err)
    }
}

impl ResponseError for SignedCookieError {}
