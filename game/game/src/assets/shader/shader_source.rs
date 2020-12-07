#![cfg(feature = "cook")]

use crate::assets::{io::HashableContent, AssetError, AssetIO, CookedShader, CookingError, ShaderType, Url};
use shaderc;

#[derive(Clone)]
pub struct ShaderSource {
    pub source_url: Url,
    pub shader_type: ShaderType,
    pub source: String,
}

impl ShaderSource {
    pub async fn load(io: &AssetIO, source_url: &Url) -> Result<(Self, String), AssetError> {
        log::debug!("[{}] Downloading...", source_url.as_str());
        let source = io.download_string(&source_url).await?;
        let ext = source_url.extension();
        let shader_type = ShaderType::from_extension(ext)?;

        let source = ShaderSource {
            source_url: source_url.clone(),
            shader_type,
            source,
        };
        let source_hash = source.source.content_hash();

        Ok((source, source_hash))
    }

    pub async fn cook(self) -> Result<CookedShader, CookingError> {
        log::debug!("[{}] Compiling...", self.source_url.as_str());
        log::trace!(
            "[{}] Source ({:?}):\n{}",
            self.source_url.as_str(),
            self.shader_type,
            self.source
        );

        let shader_type = match self.shader_type {
            ShaderType::Fragment => shaderc::ShaderKind::Fragment,
            ShaderType::Vertex => shaderc::ShaderKind::Vertex,
            ShaderType::Compute => shaderc::ShaderKind::Compute,
        };

        let mut compiler = shaderc::Compiler::new().unwrap();
        let options = shaderc::CompileOptions::new().unwrap();
        let compiled_artifact = compiler
            .compile_into_spirv(
                &self.source,
                shader_type,
                self.source_url.as_str(),
                "main",
                Some(&options),
            )
            .map_err(|err| CookingError::new(self.source_url.as_str(), err))?;

        Ok(CookedShader {
            shader_type: self.shader_type,
            binary: compiled_artifact.as_binary_u8().to_owned(),
        })
    }
}
