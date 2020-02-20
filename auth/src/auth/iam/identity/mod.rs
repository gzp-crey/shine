use serde::{Deserialize, Serialize};
use shine_core::azure_utils::{decode_safe_key, encode_safe_key};
use std::str::Utf8Error;

mod identity_data;
mod index_email;
mod index_identity;
mod index_name;
mod index_sequence;
mod manager;
mod user_identity;

pub use self::identity_data::*;
pub use self::index_email::*;
pub use self::index_identity::*;
pub use self::index_name::*;
pub use self::index_sequence::*;
pub use self::manager::*;
pub use self::user_identity::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedName(String);

impl EncodedName {
    pub fn from_raw<S: ToString>(raw: S) -> EncodedName {
        EncodedName(encode_safe_key(&raw.to_string()))
    }

    pub fn to_raw(&self) -> Result<String, Utf8Error> {
        decode_safe_key(&self.0)
    }

    pub fn prefix(&self, len: usize) -> &str {
        &self.0[..self.0.char_indices().nth(len).unwrap().0]
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedEmail(String);

impl EncodedEmail {
    pub fn from_raw<S: ToString>(raw: S) -> EncodedEmail {
        EncodedEmail(encode_safe_key(&raw.to_string()))
    }

    pub fn to_raw(&self) -> Result<String, Utf8Error> {
        decode_safe_key(&self.0)
    }

    pub fn prefix(&self, len: usize) -> &str {
        &self.0[..self.0.char_indices().nth(len).unwrap().0]
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
