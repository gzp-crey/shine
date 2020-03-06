use serde::{Deserialize, Serialize};
use shine_core::azure_utils::{decode_safe_key, encode_safe_key};
use unicode_security::GeneralSecurityProfile;
use validator::validate_email;

pub struct NameValidationError(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedName(String);

impl ValidatedName {
    pub fn from_raw(raw: &str) -> Result<ValidatedName, NameValidationError> {
        const MIN_LEN: usize = 3;
        const MAX_LEN: usize = 30;
        if !raw.chars().all(GeneralSecurityProfile::identifier_allowed) {
            Err(NameValidationError(format!("Contains disallowed characters")))
        } else if raw.chars().skip(MIN_LEN - 1).next().is_none() {
            Err(NameValidationError(format!(
                "Too short, required min length: {}",
                MIN_LEN
            )))
        } else if raw.chars().skip(MAX_LEN).next().is_some() {
            Err(NameValidationError(format!(
                "Too long, required max length: {}",
                MAX_LEN
            )))
        } else {
            Ok(ValidatedName(encode_safe_key(raw)))
        }
    }

    pub fn to_raw(&self) -> String {
        //todo: validate self during deserialize to avoid panic here
        decode_safe_key(&self.0).unwrap()
    }

    pub fn prefix(&self, len: usize) -> &str {
        &self.0[..self.0.char_indices().nth(len).unwrap().0]
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct EmailValidationError(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedEmail(String);

impl ValidatedEmail {
    pub fn from_raw(raw: &str) -> Result<ValidatedEmail, EmailValidationError> {
        if !validate_email(raw) {
            Err(EmailValidationError(format!("Invalid email")))
        } else {
            Ok(ValidatedEmail(encode_safe_key(&raw.to_string())))
        }
    }

    pub fn to_raw(&self) -> String {
        //todo: validate self during deserialize to avoid panic here
        decode_safe_key(&self.0).unwrap()
    }

    pub fn prefix(&self, len: usize) -> &str {
        &self.0[..self.0.char_indices().nth(len).unwrap().0]
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct PasswordValidationError(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedPassword(String);

impl ValidatedPassword {
    pub fn from_raw(raw: &str, /*, strength: PasswordStrength*/) -> Result<ValidatedPassword, PasswordValidationError> {
        const MIN_LEN: usize = 3;
        const MAX_LEN: usize = 30;
        if raw.chars().skip(MIN_LEN - 1).next().is_none() {
            Err(PasswordValidationError(format!(
                "Too short, required min length: {}",
                MIN_LEN
            )))
        } else if raw.chars().skip(MAX_LEN).next().is_some() {
            Err(PasswordValidationError(format!(
                "Too long, required max length: {}",
                MAX_LEN
            )))
        } else {
            Ok(ValidatedPassword(raw.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
