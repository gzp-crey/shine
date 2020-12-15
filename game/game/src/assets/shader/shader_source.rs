#![cfg(feature = "cook")]
use crate::assets::{cooker::CookingError, AssetError, AssetIO, AssetId, ContentHash, CookedShader, ShaderType, Url};

pub struct ShaderSource {
    pub source_id: AssetId,
    pub source_url: Url,
    pub shader_type: ShaderType,
    pub source: String,
}

impl ShaderSource {
    pub async fn load(io: &AssetIO, source_id: &AssetId, source_url: &Url) -> Result<(Self, ContentHash), AssetError> {
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
        let source_hash = ContentHash::from_bytes(source.source.as_bytes());

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
