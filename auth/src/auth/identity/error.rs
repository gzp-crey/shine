use actix_web::http::StatusCode;
use actix_web::ResponseError;
use argon2::Error as Argon2Error;
use azure_sdk_core::errors::AzureError;
use azure_utils::idgenerator::IdSequenceError;
use block_cipher_trait;
use block_modes;
use data_encoding;
use std::{fmt, str};

#[derive(Debug)]
pub enum IdentityError {
    /// Database related error
    DB(String),
    Encryption(String),
    InvalidName,
    InvalidEmail,
    NameTaken,
    EmailTaken,
    UserNotFound,
    PasswordNotMatching,
    LoginKeyConflict,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IdentityError::DB(ref e) => write!(f, "DB, {}", e),
            IdentityError::Encryption(ref e) => write!(f, "Encryption, encryption failed: {}", e),
            IdentityError::InvalidName => write!(f, "Invalid user name"),
            IdentityError::InvalidEmail => write!(f, "Invalid email"),
            IdentityError::NameTaken => write!(f, "User name already taken"),
            IdentityError::EmailTaken => write!(f, "Email already taken"),
            IdentityError::LoginKeyConflict => write!(f, "Login key already in use"),
            IdentityError::UserNotFound | IdentityError::PasswordNotMatching => write!(f, "Invalid user or password"),
        }
    }
}

impl ResponseError for IdentityError {
    fn status_code(&self) -> StatusCode {
        match *self {
            IdentityError::InvalidEmail => StatusCode::BAD_REQUEST,
            IdentityError::InvalidName => StatusCode::BAD_REQUEST,
            IdentityError::Encryption(_) => StatusCode::BAD_REQUEST,
            IdentityError::NameTaken => StatusCode::CONFLICT,
            IdentityError::EmailTaken => StatusCode::CONFLICT,
            IdentityError::LoginKeyConflict => StatusCode::TOO_MANY_REQUESTS,
            IdentityError::UserNotFound | IdentityError::PasswordNotMatching => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<AzureError> for IdentityError {
    fn from(err: AzureError) -> IdentityError {
        IdentityError::DB(format!("{:?}", err))
    }
}

impl From<IdSequenceError> for IdentityError {
    fn from(err: IdSequenceError) -> IdentityError {
        match err {
            IdSequenceError::DB(e) => IdentityError::DB(e),
            IdSequenceError::SequenceEnded => IdentityError::DB(format!("ID sequence out of values")),
        }
    }
}

impl From<Argon2Error> for IdentityError {
    fn from(err: Argon2Error) -> IdentityError {
        IdentityError::Encryption(err.to_string())
    }
}

impl From<data_encoding::DecodeError> for IdentityError {
    fn from(err: data_encoding::DecodeError) -> IdentityError {
        IdentityError::Encryption(err.to_string())
    }
}

impl From<block_modes::InvalidKeyIvLength> for IdentityError {
    fn from(err: block_modes::InvalidKeyIvLength) -> IdentityError {
        IdentityError::Encryption(err.to_string())
    }
}

impl From<block_modes::BlockModeError> for IdentityError {
    fn from(err: block_modes::BlockModeError) -> IdentityError {
        IdentityError::Encryption(err.to_string())
    }
}

impl From<block_cipher_trait::InvalidKeyLength> for IdentityError {
    fn from(err: block_cipher_trait::InvalidKeyLength) -> IdentityError {
        IdentityError::Encryption(err.to_string())
    }
}

impl From<str::Utf8Error> for IdentityError {
    fn from(err: str::Utf8Error) -> IdentityError {
        IdentityError::Encryption(err.to_string())
    }
}
