use crate::resources::ResourceId;
use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ECSError {
    #[error("Resource id error: {0:?}")]
    ResourceId(#[from] Box<dyn std::error::Error>),

    #[error("Resource store for {0} already registered")]
    ResourceAlreadyRegistered(Cow<'static, str>),

    #[error("Resource store for {0} not registered")]
    ResourceTypeNotFound(Cow<'static, str>),

    #[error("Resource {1:?} not found ({0})")]
    ResourceNotFound(Cow<'static, str>, ResourceId),

    #[error("Resource handle was invalidated")]
    ResourceExpired,

    #[error("Invalid resource claim")]
    ResourceClaimError,

    #[error("System lock could not be claimed")]
    SystemLockError,
}
