mod small_string_id;
pub use self::small_string_id::*;
mod interval_id;
pub use self::interval_id::*;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdError {
    #[error("Could not parse [{0}] as id")]
    ParseError(String),
}
