use serde::{Deserialize, Serialize};
use shine_core::azure_utils::{decode_safe_key, encode_safe_key};
use unicode_security::GeneralSecurityProfile;
use validator::validate_email;

#[derive(Debug, Clone)]
pub enum NameValidationError {
    TooShort,
    TooLong,
    InvalidCharacter(Vec<char>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedName(String);

impl ValidatedName {
    pub const MIN_LEN: usize = 3;
    pub const MAX_LEN: usize = 30;

    pub fn from_raw(raw: &str) -> Result<ValidatedName, NameValidationError> {
        let mut invalid_character: Vec<char> = raw.chars().filter(|c| c.identifier_allowed()).collect();
        invalid_character.sort_by(|a, b| b.cmp(a));
        invalid_character.dedup();

        if !invalid_character.is_empty() {
            Err(NameValidationError::InvalidCharacter(invalid_character))
        } else if raw.chars().skip(Self::MIN_LEN - 1).next().is_none() {
            Err(NameValidationError::TooShort)
        } else if raw.chars().skip(Self::MAX_LEN).next().is_some() {
            Err(NameValidationError::TooLong)
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

#[derive(Debug, Clone)]
pub enum EmailValidationError {
    InvalidFormat,
    UnsupportedDomain(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedEmail(String);

impl ValidatedEmail {
    pub fn from_raw(raw: &str) -> Result<ValidatedEmail, EmailValidationError> {
        if !validate_email(raw) {
            Err(EmailValidationError::InvalidFormat)
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

#[derive(Debug, Clone)]
pub enum PasswordValidationError {
    TooShort,
    TooLong,
    TooWeek,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatedPassword(String);

impl ValidatedPassword {
    pub const MIN_LEN: usize = 3;
    pub const MAX_LEN: usize = 30;

    pub fn from_raw(raw: &str, /*, strength: PasswordStrength*/) -> Result<ValidatedPassword, PasswordValidationError> {
        if raw.chars().skip(Self::MIN_LEN - 1).next().is_none() {
            Err(PasswordValidationError::TooShort)
        } else if raw.chars().skip(Self::MAX_LEN).next().is_some() {
            Err(PasswordValidationError::TooLong)
        } else {
            Ok(ValidatedPassword(raw.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
