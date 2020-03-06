use super::iam::{
    identity::{EmailValidationError, Identity, NameValidationError, PasswordValidationError, UserIdentity},
    role::InheritedRoles,
    IAMError,
};
use shine_core::kernel::identity::UserId;
use shine_core::kernel::response::APIError;
use std::collections::HashSet;
use std::iter::FromIterator;

pub(crate) fn create_user_id(user: UserIdentity, roles: InheritedRoles) -> Result<UserId, IAMError> {
    let roles = HashSet::from_iter(roles.into_iter().map(|r| r.role));
    let data = user.into_data();
    let user_name = data.core.name.to_raw();
    Ok(UserId::new(data.core.id, user_name, roles))
}

impl From<NameValidationError> for APIError {
    fn from(err: NameValidationError) -> APIError {
        APIError::BadRequest(format!("Invalid name: {}", err.0))
    }
}

impl From<EmailValidationError> for APIError {
    fn from(err: EmailValidationError) -> APIError {
        APIError::BadRequest(format!("Invalid email: {}", err.0))
    }
}

impl From<PasswordValidationError> for APIError {
    fn from(err: PasswordValidationError) -> APIError {
        APIError::BadRequest(format!("Invalid password: {}", err.0))
    }
}
