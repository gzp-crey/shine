use super::IAMError;
use serde::{Deserialize, Serialize};
use shine_core::azure_utils::{decode_safe_key, encode_safe_key};
use std::str::Utf8Error;
use unicode_security::GeneralSecurityProfile;
use validator::validate_email;

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
    pub fn from_raw(raw: &str) -> Result<EncodedName, IAMError> {
        const MIN_LEN: usize = 3;
        const MAX_LEN: usize = 30;
        if !raw.chars().all(GeneralSecurityProfile::identifier_allowed) {
            Err(IAMError::NameInvalid(format!("Contains disallowed characters")))
        } else if raw.chars().skip(MIN_LEN - 1).next().is_none() {
            Err(IAMError::NameInvalid(format!(
                "Too short, required min length: {}",
                MIN_LEN
            )))
        } else if raw.chars().skip(MAX_LEN).next().is_some() {
            Err(IAMError::NameInvalid(format!(
                "Too long, required max length: {}",
                MAX_LEN
            )))
        } else {
            Ok(EncodedName(encode_safe_key(raw)))
        }
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
    pub fn from_raw(raw: &str) -> Result<EncodedEmail, IAMError> {
        if !validate_email(raw) {
            Err(IAMError::EmailInvalid(format!("Invalid email")))
        } else {
            Ok(EncodedEmail(encode_safe_key(&raw.to_string())))
        }
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
