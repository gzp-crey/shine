mod dummy_cooker;
pub use self::dummy_cooker::*;
mod cooking_error;
pub use self::cooking_error::*;

use crate::assets::{AssetId, ContentHash, Url, UrlError};
use std::future::Future;

pub enum Naming {
    Soft(String, String),
    Hard(String, String),
}

impl Naming {
    pub fn soft(schema: &str, ext: &str) -> Naming {
        Naming::Soft(schema.to_owned(), ext.to_owned())
    }

    pub fn hard(schema: &str, ext: &str) -> Naming {
        Naming::Hard(schema.to_owned(), ext.to_owned())
    }

    pub fn is_soft(&self) -> bool {
        if let Naming::Soft(..) = self {
            true
        } else {
            false
        }
    }

    pub fn is_hard(&self) -> bool {
        if let Naming::Hard(..) = self {
            true
        } else {
            false
        }
    }

    pub fn to_url(&self, source_id: &AssetId, hash: &ContentHash) -> Result<Url, UrlError> {
        match self {
            Naming::Hard(scheme, ext) => {
                let hashed_path = hash.to_path();
                Url::parse(&format!("hash-{}://{}.{}", scheme, hashed_path, ext))
            }
            Naming::Soft(scheme, ext) => {
                Url::parse(&format!("{}://{}", scheme, source_id.as_str()))?.set_extension(&ext)
            }
        }
    }
}

/// Trait to cook shader
pub trait ShaderCooker<'a> {
    type ShaderFuture: 'a + Future<Output = Result<Url, CookingError>>;

    fn cook_shader(&self, source_id: AssetId, naming: Naming) -> Self::ShaderFuture;
}

/// Trait to cook pipeline
pub trait PipelineCooker<'a>: ShaderCooker<'a> {
    type PipelineFuture: 'a + Future<Output = Result<Url, CookingError>>;

    fn cook_pipeline(&self, source_id: AssetId, naming: Naming) -> Self::PipelineFuture;
}

/// Trait to cook texzure
pub trait TextureCooker<'a> {
    type TextureFuture: 'a + Future<Output = Result<Url, CookingError>>;

    fn cook_texture(&self, source_id: AssetId, naming: Naming) -> Self::TextureFuture;
}

/// Trait to cook model
pub trait ModelCooker<'a> {
    type ModelFuture: 'a + Future<Output = Result<Url, CookingError>>;

    fn cook_model(&self, source_id: AssetId, naming: Naming) -> Self::ModelFuture;
}
