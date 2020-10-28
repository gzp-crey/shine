use crate::resources::ResourceId;
use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ECSError {
    #[error("Resource {0} {1:?} not found")]
    ResourceNotFound(Cow<'static, str>, ResourceId),
    #[error("Invalid resource claim")]
    ResourceClaimError,
}
