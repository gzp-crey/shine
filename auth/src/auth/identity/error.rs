use actix_web::http::StatusCode;
use actix_web::ResponseError;
use argon2::Error as Argon2Error;
use azure_sdk_core::errors::AzureError;
use azure_utils::idgenerator::IdSequenceError;
use std::fmt;

#[derive(Debug)]
pub enum IdentityError {
    /// Database related error
    DB(String),
    InvalidPassword(String),
    InvalidName,    
    InvalidEmail,
    NameTaken,
    EmailTaken,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IdentityError::DB(ref e) => write!(f, "DB, {}", e),
            IdentityError::InvalidPassword(ref e) => write!(f, "InvalidPassword, pasword encryption failed: {}", e),
            IdentityError::InvalidName => write!(f, "Invalid user name"),
            IdentityError::InvalidEmail => write!(f, "Invalid email"),
            IdentityError::NameTaken => write!(f, "User name already taken"),
            IdentityError::EmailTaken => write!(f, "Email already taken"),
        }
    }
}

impl ResponseError for IdentityError {
    fn status_code(&self) -> StatusCode {
        match *self {
            IdentityError::InvalidEmail => StatusCode::BAD_REQUEST,
            IdentityError::InvalidName => StatusCode::BAD_REQUEST,
            IdentityError::InvalidPassword(_) => StatusCode::BAD_REQUEST,
            IdentityError::NameTaken => StatusCode::CONFLICT,
            IdentityError::EmailTaken => StatusCode::CONFLICT,
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
        IdentityError::InvalidPassword(err.to_string())
    }
}
