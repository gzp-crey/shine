use actix_web::http::StatusCode;
use actix_web::ResponseError;
use argon2::Error as Argon2Error;
use azure_sdk_core::errors::AzureError;
use data_encoding;
use shine_core::{
    backoff::BackoffError, idgenerator::IdSequenceError, iplocation::IpLocationError, requestinfo::RequestInfoError,
};
use std::{fmt, str};

#[derive(Debug)]
pub enum IAMError {
    /// Database related error
    DB(String),
    Request(String),
    InvalidName,
    InvalidEmail,
    SequenceIdTaken,
    NameTaken,
    EmailTaken,
    IdentityNotFound,
    PasswordNotMatching,
    IdentityIdConflict,
    SessionKeyConflict,
    SessionRequired,
    SessionExpired,
}

impl IAMError {
    pub fn into_backoff(self) -> BackoffError<IAMError> {
        match self {
            IAMError::IdentityIdConflict => BackoffError::Transient(IAMError::IdentityIdConflict),
            IAMError::SessionKeyConflict => BackoffError::Transient(IAMError::SessionKeyConflict),
            e => BackoffError::Permanent(e),
        }
    }
}

impl fmt::Display for IAMError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            IAMError::DB(ref e) => write!(f, "DB, {}", e),
            IAMError::Request(ref e) => write!(f, "Request, {}", e),
            IAMError::InvalidName => write!(f, "Invalid name"),
            IAMError::InvalidEmail => write!(f, "Invalid email"),
            IAMError::SequenceIdTaken => write!(f, "Sequence id already taken"),
            IAMError::NameTaken => write!(f, "Name already taken"),
            IAMError::EmailTaken => write!(f, "Email already taken"),
            IAMError::IdentityIdConflict => write!(f, "Identity id already in use"),
            IAMError::SessionKeyConflict => write!(f, "Session key already in use"),
            IAMError::IdentityNotFound => write!(f, "Identity not found"),
            IAMError::PasswordNotMatching => write!(f, "Invalid user or password"),
            IAMError::SessionRequired => write!(f, "Login required"),
            IAMError::SessionExpired => write!(f, "Login expired"),
        }
    }
}

impl ResponseError for IAMError {
    fn status_code(&self) -> StatusCode {
        match *self {
            IAMError::InvalidEmail => StatusCode::BAD_REQUEST,
            IAMError::Request(_) => StatusCode::BAD_REQUEST,
            IAMError::InvalidName => StatusCode::BAD_REQUEST,
            IAMError::SequenceIdTaken => StatusCode::CONFLICT,
            IAMError::NameTaken => StatusCode::CONFLICT,
            IAMError::EmailTaken => StatusCode::CONFLICT,
            IAMError::IdentityIdConflict => StatusCode::TOO_MANY_REQUESTS,
            IAMError::SessionKeyConflict => StatusCode::TOO_MANY_REQUESTS,
            IAMError::IdentityNotFound | IAMError::PasswordNotMatching => StatusCode::FORBIDDEN,
            IAMError::SessionRequired => StatusCode::UNAUTHORIZED,
            IAMError::SessionExpired => StatusCode::UNAUTHORIZED,
            IAMError::DB(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<AzureError> for IAMError {
    fn from(err: AzureError) -> IAMError {
        IAMError::DB(format!("{:?}", err))
    }
}

impl From<IpLocationError> for IAMError {
    fn from(err: IpLocationError) -> IAMError {
        IAMError::DB(format!("{:?}", err))
    }
}

impl From<IdSequenceError> for IAMError {
    fn from(err: IdSequenceError) -> IAMError {
        match err {
            IdSequenceError::SequenceEnded => IAMError::DB(format!("ID sequence out of values")),
            e => IAMError::DB(format!("Sequence error: {}", e)),
        }
    }
}

impl From<RequestInfoError> for IAMError {
    fn from(err: RequestInfoError) -> IAMError {
        IAMError::Request(err.to_string())
    }
}

impl From<Argon2Error> for IAMError {
    fn from(err: Argon2Error) -> IAMError {
        IAMError::Request(err.to_string())
    }
}

impl From<data_encoding::DecodeError> for IAMError {
    fn from(err: data_encoding::DecodeError) -> IAMError {
        IAMError::Request(err.to_string())
    }
}

impl From<str::Utf8Error> for IAMError {
    fn from(err: str::Utf8Error) -> IAMError {
        IAMError::Request(err.to_string())
    }
}
