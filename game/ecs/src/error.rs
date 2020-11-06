use crate::resources::ResourceId;
use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ECSError {
    #[error("Resource id error: {0:?}")]
    ResourceId(#[from] Box<dyn std::error::Error>),

    #[error("Resource with {0} type not found ({1:?})")]
    ResourceNotFound(Cow<'static, str>, Option<ResourceId>),

    #[error("Resource handle was invalidated")]
    ResourceExpired,

    #[error("Invalid resource claim")]
    ResourceClaimError,
}
