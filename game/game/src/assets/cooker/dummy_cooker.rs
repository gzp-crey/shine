use crate::assets::{
    cooker::{CookingError, ModelCooker, Naming, PipelineCooker, ShaderCooker, TextureCooker},
    AssetId, ShaderType, Url,
};
use std::{future::Future, pin::Pin};

/// A dummy cooker that does nothing, but returns some dummy cooked id
pub struct DummyCooker;

impl<'a> ShaderCooker<'a> for DummyCooker {
    type ShaderFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_shader(&self, _sh: ShaderType, id: AssetId, _naming: Naming) -> Self::ShaderFuture {
        Box::pin(async move {
            let url = Url::parse("shader://").map_err(|err| CookingError::from_err(id.to_string(), err))?;
            Ok(id
                .to_url(&url)
                .map_err(|err| CookingError::from_err(id.to_string(), err))?)
        })
    }
}

impl<'a> PipelineCooker<'a> for DummyCooker {
    type PipelineFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_pipeline(&self, id: AssetId, _naming: Naming) -> Self::PipelineFuture {
        Box::pin(async move {
            let url = Url::parse("pipeline://").map_err(|err| CookingError::from_err(id.to_string(), err))?;
            Ok(id
                .to_url(&url)
                .map_err(|err| CookingError::from_err(id.to_string(), err))?)
        })
    }
}

impl<'a> TextureCooker<'a> for DummyCooker {
    type TextureFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_texture(&self, id: AssetId, _naming: Naming) -> Self::TextureFuture {
        Box::pin(async move {
            let url = Url::parse("texture://").map_err(|err| CookingError::from_err(id.to_string(), err))?;
            Ok(id
                .to_url(&url)
                .map_err(|err| CookingError::from_err(id.to_string(), err))?)
        })
    }
}

impl<'a> ModelCooker<'a> for DummyCooker {
    type ModelFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_model(&self, id: AssetId, _naming: Naming) -> Self::ModelFuture {
        Box::pin(async move {
            let url = Url::parse("model://").map_err(|err| CookingError::from_err(id.to_string(), err))?;
            Ok(id
                .to_url(&url)
                .map_err(|err| CookingError::from_err(id.to_string(), err))?)
        })
    }
}
