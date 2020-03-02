use azure_sdk_core::errors::AzureError;
use gremlin_client::GremlinError;
use shine_core::{
    backoff::BackoffError,
    idgenerator::IdSequenceError,
    iplocation::IpLocationError,
    kernel::anti_forgery::AntiForgeryError,
    kernel::response::{APIError, PageError},
    requestinfo::RequestInfoError,
};

#[derive(Debug)]
pub enum IAMError {
    /// Database related error
    Internal(String),
    BadRequest(String),

    NameInvalid(String),
    NameTaken,
    EmailInvalid(String),
    EmailTaken,

    SequenceIdTaken,
    IdentityIdConflict,
    IdentityNotFound,
    PasswordNotMatching,
    SessionRequired,
    SessionExpired,
    SessionKeyConflict,

    RoleNotFound,
    RoleTaken,
    HasRoleTaken,
    HasRoleCycle(Vec<String>),

    InsufficientPermission,
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

impl From<IAMError> for PageError {
    fn from(err: IAMError) -> PageError {
        PageError::Internal(format!("{:?}", err))
        /*match *self {
            IAMError::Internal(_) => APIError::Internal,
            IAMError::BadRequest(r) => APIError::BAD_REQUEST,

            IAMError::NameInvalid(_) => StatusCode::BAD_REQUEST,
            IAMError::NameTaken => StatusCode::CONFLICT,
            IAMError::EmailInvalid(_) => StatusCode::BAD_REQUEST,
            IAMError::EmailTaken => StatusCode::CONFLICT,

            IAMError::SequenceIdTaken => StatusCode::CONFLICT,
            IAMError::IdentityIdConflict => StatusCode::TOO_MANY_REQUESTS,
            IAMError::IdentityNotFound => StatusCode::FORBIDDEN,
            IAMError::PasswordNotMatching => StatusCode::FORBIDDEN,
            IAMError::SessionRequired => StatusCode::UNAUTHORIZED,
            IAMError::SessionExpired => StatusCode::UNAUTHORIZED,
            IAMError::SessionKeyConflict => StatusCode::TOO_MANY_REQUESTS,

            IAMError::RoleNotFound => StatusCode::NOT_FOUND,
            IAMError::RoleTaken => StatusCode::CONFLICT,
            IAMError::HasRoleTaken => StatusCode::CONFLICT,
            IAMError::HasRoleCycle(_) => StatusCode::CONFLICT,

            IAMError::InsufficientPermission => StatusCode::FORBIDDEN,
        }*/
    }
}

impl From<IAMError> for APIError {
    fn from(err: IAMError) -> APIError {
        APIError::Internal(format!("{:?}", err))
        /*match *self {
            IAMError::Internal(_) => APIError::Internal,
            IAMError::BadRequest(r) => APIError::BAD_REQUEST,

            IAMError::NameInvalid(_) => StatusCode::BAD_REQUEST,
            IAMError::NameTaken => StatusCode::CONFLICT,
            IAMError::EmailInvalid(_) => StatusCode::BAD_REQUEST,
            IAMError::EmailTaken => StatusCode::CONFLICT,

            IAMError::SequenceIdTaken => StatusCode::CONFLICT,
            IAMError::IdentityIdConflict => StatusCode::TOO_MANY_REQUESTS,
            IAMError::IdentityNotFound => StatusCode::FORBIDDEN,
            IAMError::PasswordNotMatching => StatusCode::FORBIDDEN,
            IAMError::SessionRequired => StatusCode::UNAUTHORIZED,
            IAMError::SessionExpired => StatusCode::UNAUTHORIZED,
            IAMError::SessionKeyConflict => StatusCode::TOO_MANY_REQUESTS,

            IAMError::RoleNotFound => StatusCode::NOT_FOUND,
            IAMError::RoleTaken => StatusCode::CONFLICT,
            IAMError::HasRoleTaken => StatusCode::CONFLICT,
            IAMError::HasRoleCycle(_) => StatusCode::CONFLICT,

            IAMError::InsufficientPermission => StatusCode::FORBIDDEN,
        }*/
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

impl From<AntiForgeryError> for IAMError {
    fn from(_err: AntiForgeryError) -> IAMError {
        IAMError::BadRequest(format!("AF check failed"))
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
