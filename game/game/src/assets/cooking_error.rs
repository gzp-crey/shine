#![cfg(feature = "cook")]

use std::error::Error as StdError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Error during cooking {}", content_id)]
pub struct CookingError {
    content_id: String,
    source: Box<dyn 'static + Sync + Send + StdError>,
}

impl CookingError {
    pub fn new<S: ToString, E: 'static + Sync + Send + StdError>(content_id: S, error: E) -> Self {
        Self {
            content_id: content_id.to_string(),
            source: error.into(),
        }
    }
}
