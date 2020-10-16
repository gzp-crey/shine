use crate::ecs::resources::ResourceHandle;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ECSError {
    #[error("Resource {0} not found")]
    ResourceNotFound(ResourceHandle),
    #[error("Invalid resource claim")]
    ResourceClaimError,
}
