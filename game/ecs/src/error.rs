use crate::resources::ResourceId;
use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ECSError {
    #[error("Resource id error: {0:?}")]
    ResourceId(#[from] Box<dyn std::error::Error>),

    #[error("Resource store for {0} not found")]
    ResourceTypeNotFound(Cow<'static, str>),

    #[error("Resource handle belongs to a different store")]
    ResourceHandleAlien,

    #[error("Resource of {0} not found by handle")]
    ResourceHandleNotFound(Cow<'static, str>),

    #[error("Resource {0} {1:?} not found")]
    ResourceNotFound(Cow<'static, str>, ResourceId),

    #[error("Invalid resource claim")]
    ResourceClaimError,
}
