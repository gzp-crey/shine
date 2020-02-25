use actix_web::http::StatusCode;
use actix_web::ResponseError;
use azure_sdk_core::errors::AzureError;
use gremlin_client::GremlinError;
use shine_core::{
    backoff::BackoffError, idgenerator::IdSequenceError, iplocation::IpLocationError, requestinfo::RequestInfoError,
};
use std::fmt;

#[derive(Debug)]
pub enum IAMError {
    /// Database related error
    Internal(String),
    BadRequest(String),
    InvalidName(String),
    InvalidEmail(String),
    SequenceIdTaken,
    NameTaken,
    EmailTaken,
    RoleNotFound,
    RoleTaken,
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
            IAMError::Internal(ref e) => write!(f, "Internal error: {}", e),
            IAMError::BadRequest(ref e) => write!(f, "BadRequest, {}", e),
            IAMError::InvalidName(ref e) => write!(f, "Invalid name: {}", e),
            IAMError::InvalidEmail(ref e) => write!(f, "Invalid email: {}", e),
            IAMError::SequenceIdTaken => write!(f, "Sequence id already taken"),
            IAMError::NameTaken => write!(f, "Name already taken"),
            IAMError::EmailTaken => write!(f, "Email already taken"),
            IAMError::IdentityIdConflict => write!(f, "Identity id already in use"),
            IAMError::SessionKeyConflict => write!(f, "Session key already in use"),
            IAMError::RoleNotFound => write!(f, "Role not found"),
            IAMError::RoleTaken => write!(f, "Role already taken"),
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
            IAMError::InvalidName(_) => StatusCode::BAD_REQUEST,
            IAMError::InvalidEmail(_) => StatusCode::BAD_REQUEST,
            IAMError::BadRequest(_) => StatusCode::BAD_REQUEST,
            IAMError::SequenceIdTaken => StatusCode::CONFLICT,
            IAMError::NameTaken => StatusCode::CONFLICT,
            IAMError::EmailTaken => StatusCode::CONFLICT,
            IAMError::IdentityIdConflict => StatusCode::TOO_MANY_REQUESTS,
            IAMError::SessionKeyConflict => StatusCode::TOO_MANY_REQUESTS,
            IAMError::IdentityNotFound | IAMError::PasswordNotMatching => StatusCode::FORBIDDEN,
            IAMError::SessionRequired => StatusCode::UNAUTHORIZED,
            IAMError::SessionExpired => StatusCode::UNAUTHORIZED,
            IAMError::RoleNotFound => StatusCode::NOT_FOUND,
            IAMError::RoleTaken => StatusCode::CONFLICT,
            IAMError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<AzureError> for IAMError {
    fn from(err: AzureError) -> IAMError {
        IAMError::Internal(format!("{:?}", err))
    }
}

impl From<IpLocationError> for IAMError {
    fn from(err: IpLocationError) -> IAMError {
        IAMError::Internal(format!("{:?}", err))
    }
}

impl From<IdSequenceError> for IAMError {
    fn from(err: IdSequenceError) -> IAMError {
        match err {
            IdSequenceError::SequenceEnded => IAMError::Internal(format!("ID sequence out of values")),
            e => IAMError::Internal(format!("Sequence error: {}", e)),
        }
    }
}

impl From<RequestInfoError> for IAMError {
    fn from(err: RequestInfoError) -> IAMError {
        IAMError::BadRequest(err.to_string())
    }
}

impl From<GremlinError> for IAMError {
    fn from(err: GremlinError) -> IAMError {
        IAMError::Internal(err.to_string())
    }
}
