use crate::assets::{
    cooker::{ContentHash, CookingError, ModelCooker, Naming, PipelineCooker, ShaderCooker, TextureCooker},
    AssetId, Url,
};
use std::{future::Future, pin::Pin};

/// A dummy cooker that does nothing, but returns some dummy cooked id
pub struct DummyCooker;

impl<'a> ShaderCooker<'a> for DummyCooker {
    type ShaderFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_shader(&self, source_id: AssetId, naming: Naming) -> Self::ShaderFuture {
        Box::pin(async move {
            Ok(naming
                .to_url(&source_id, &ContentHash::from_str(source_id.as_str()))
                .map_err(|err| CookingError::from_err(source_id.to_string(), err))?)
        })
    }
}

impl<'a> PipelineCooker<'a> for DummyCooker {
    type PipelineFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_pipeline(&self, source_id: AssetId, naming: Naming) -> Self::PipelineFuture {
        Box::pin(async move {
            Ok(naming
                .to_url(&source_id, &ContentHash::from_str(source_id.as_str()))
                .map_err(|err| CookingError::from_err(source_id.to_string(), err))?)
        })
    }
}

impl<'a> TextureCooker<'a> for DummyCooker {
    type TextureFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_texture(&self, source_id: AssetId, naming: Naming) -> Self::TextureFuture {
        Box::pin(async move {
            Ok(naming
                .to_url(&source_id, &ContentHash::from_str(source_id.as_str()))
                .map_err(|err| CookingError::from_err(source_id.to_string(), err))?)
        })
    }
}

impl<'a> ModelCooker<'a> for DummyCooker {
    type ModelFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_model(&self, source_id: AssetId, naming: Naming) -> Self::ModelFuture {
        Box::pin(async move {
            Ok(naming
                .to_url(&source_id, &ContentHash::from_str(source_id.as_str()))
                .map_err(|err| CookingError::from_err(source_id.to_string(), err))?)
        })
    }
}
