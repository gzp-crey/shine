use std::{error::Error, fmt};

/// Wrap a string as an error for convenience
#[derive(Debug)]
pub struct DisplayError(pub String);

impl fmt::Display for DisplayError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Error for DisplayError {}
