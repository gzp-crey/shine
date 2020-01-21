use actix_web::http::StatusCode;
use actix_web::ResponseError;
use argon2::Error as Argon2Error;
use azure_sdk_core::errors::AzureError;
use data_encoding;
use shine_core::{backoff::BackoffError, idgenerator::IdSequenceError};
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
    IdentityNotFound,
    PasswordNotMatching,
    IdentityIdConflict,
    SessionKeyConflict,
    SessionRequired,
    SessionExpired,
}

impl IdentityError {
    pub fn into_backoff(self) -> BackoffError<IdentityError> {
        match self {
            IdentityError::IdentityIdConflict => BackoffError::Transient(IdentityError::IdentityIdConflict),
            IdentityError::SessionKeyConflict => BackoffError::Transient(IdentityError::SessionKeyConflict),
            e => BackoffError::Permanent(e),
        }
    }
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IdentityError::DB(ref e) => write!(f, "DB, {}", e),
            IdentityError::Encryption(ref e) => write!(f, "Encryption, encryption failed: {}", e),
            IdentityError::InvalidName => write!(f, "Invalid name"),
            IdentityError::InvalidEmail => write!(f, "Invalid email"),
            IdentityError::NameTaken => write!(f, "Name already taken"),
            IdentityError::EmailTaken => write!(f, "Email already taken"),
            IdentityError::IdentityIdConflict => write!(f, "Identity id already in use"),
            IdentityError::SessionKeyConflict => write!(f, "Login key already in use"),
            IdentityError::IdentityNotFound => write!(f, "Identity not found"),
            IdentityError::PasswordNotMatching => write!(f, "Invalid user or password"),
            IdentityError::SessionRequired => write!(f, "Login required"),
            IdentityError::SessionExpired => write!(f, "Login expired"),
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
            IdentityError::IdentityIdConflict => StatusCode::TOO_MANY_REQUESTS,
            IdentityError::SessionKeyConflict => StatusCode::TOO_MANY_REQUESTS,
            IdentityError::IdentityNotFound | IdentityError::PasswordNotMatching => StatusCode::FORBIDDEN,
            IdentityError::SessionRequired => StatusCode::UNAUTHORIZED,
            IdentityError::SessionExpired => StatusCode::UNAUTHORIZED,
            IdentityError::DB(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
            IdSequenceError::SequenceEnded => IdentityError::DB(format!("ID sequence out of values")),
            e => IdentityError::DB(format!("Sequence error: {}", e)),
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

impl From<str::Utf8Error> for IdentityError {
    fn from(err: str::Utf8Error) -> IdentityError {
        IdentityError::Encryption(err.to_string())
    }
}
