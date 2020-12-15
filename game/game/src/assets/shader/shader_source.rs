#![cfg(feature = "cook")]
use crate::assets::{
    cooker::DummyCooker, io::HashableContent, AssetError, AssetIO, AssetId, CookedShader, CookingError, ShaderType, Url,
};
use std::{future::Future, pin::Pin};

pub struct ShaderSource {
    pub source_id: AssetId,
    pub source_url: Url,
    pub shader_type: ShaderType,
    pub source: String,
}

impl ShaderSource {
    pub async fn load(io: &AssetIO, source_id: &AssetId, source_url: &Url) -> Result<(Self, String), AssetError> {
        if source_id.is_relative() {
            return Err(AssetError::InvalidAssetId(format!(
                "Absolute id required: {}",
                source_id.as_str()
            )));
        }

        log::debug!("[{}] Downloading from {}...", source_id.as_str(), source_url.as_str());
        let source = io.download_string(&source_url).await?;
        let ext = source_url.extension();
        let shader_type = ShaderType::from_extension(ext)?;

        let source = ShaderSource {
            source_id: source_id.clone(),
            source_url: source_url.clone(),
            shader_type,
            source,
        };
        let source_hash = source.source.content_hash();

        Ok((source, source_hash))
    }

    pub async fn cook(self) -> Result<CookedShader, CookingError> {
        log::debug!("[{}] Compiling...", self.source_id.as_str());

        let ShaderSource {
            source_id,
            shader_type,
            source,
            ..
        } = self;

        log::trace!("[{}] Source ({:?}):\n{}", source_id.as_str(), shader_type, source);

        let shader_kind = match shader_type {
            ShaderType::Fragment => shaderc::ShaderKind::Fragment,
            ShaderType::Vertex => shaderc::ShaderKind::Vertex,
            ShaderType::Compute => shaderc::ShaderKind::Compute,
        };

        let mut compiler = shaderc::Compiler::new().unwrap();
        let options = shaderc::CompileOptions::new().unwrap();
        let compiled_artifact = compiler
            .compile_into_spirv(&source, shader_kind, source_id.as_str(), "main", Some(&options))
            .map_err(|err| CookingError::from_err(source_id.as_str(), err))?;

        Ok(CookedShader {
            shader_type,
            binary: compiled_artifact.as_binary_u8().to_owned(),
        })
    }
}

/// Trait to cook shader
pub trait ShaderCooker<'a> {
    type Fut: 'a + Future<Output = Result<Url, CookingError>>;

    fn cook_shader(&self, sh: ShaderType, id: AssetId) -> Self::Fut;
}

impl<'a, F, Fut> ShaderCooker<'a> for F
where
    Fut: 'a + Future<Output = Result<Url, CookingError>>,
    F: 'a + Fn(ShaderType, AssetId) -> Fut,
{
    type Fut = Fut;

    fn cook_shader(&self, sh: ShaderType, id: AssetId) -> Fut {
        (self)(sh, id)
    }
}

impl<'a> ShaderCooker<'a> for DummyCooker {
    type Fut = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_shader(&self, _sh: ShaderType, id: AssetId) -> Self::Fut {
        Box::pin(async move {
            let url = Url::parse("shader://").map_err(|err| CookingError::from_err(id.to_string(), err))?;
            Ok(id
                .to_url(&url)
                .map_err(|err| CookingError::from_err(id.to_string(), err))?)
        })
    }
}