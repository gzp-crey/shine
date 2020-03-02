use super::iam::{
    identity::{Identity, UserIdentity},
    role::InheritedRoles,
    IAMError,
};
use shine_core::kernel::identity::UserId;
use std::collections::HashSet;
use std::iter::FromIterator;

pub(crate) fn create_user_id(user: UserIdentity, roles: InheritedRoles) -> Result<UserId, IAMError> {
    let roles = HashSet::from_iter(roles.into_iter().map(|r| r.role));
    let data = user.into_data();
    let user_name = data
        .core
        .name
        .to_raw()
        .map_err(|err| IAMError::Internal(format!("Name decript error: {}", err)))?;
    Ok(UserId::new(data.core.id, user_name, roles))
}
