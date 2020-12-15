mod dummy_cooker;
pub use self::dummy_cooker::*;
mod cooking_error;
pub use self::cooking_error::*;

use crate::assets::{AssetId, ShaderType, Url};
use std::future::Future;

pub enum Naming {
    Soft,
    Hard,
}

/// Trait to cook shader
pub trait ShaderCooker<'a> {
    type ShaderFuture: 'a + Future<Output = Result<Url, CookingError>>;

    fn cook_shader(&self, sh: ShaderType, id: AssetId, naming: Naming) -> Self::ShaderFuture;
}

/// Trait to cook pipeline
pub trait PipelineCooker<'a>: ShaderCooker<'a> {
    type PipelineFuture: 'a + Future<Output = Result<Url, CookingError>>;

    fn cook_pipeline(&self, id: AssetId, naming: Naming) -> Self::PipelineFuture;
}

/// Trait to cook texzure
pub trait TextureCooker<'a> {
    type TextureFuture: 'a + Future<Output = Result<Url, CookingError>>;

    fn cook_texture(&self, id: AssetId, naming: Naming) -> Self::TextureFuture;
}

/// Trait to cook model
pub trait ModelCooker<'a> {
    type ModelFuture: 'a + Future<Output = Result<Url, CookingError>>;

    fn cook_model(&self, id: AssetId, naming: Naming) -> Self::ModelFuture;
}
