#![cfg(feature = "cook")]

use crate::assets::{io::HashableContent, AssetError, AssetIO, CookedShader, CookingError, Url};
use serde::{Deserialize, Serialize};
use shaderc;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

#[derive(Clone)]
pub struct ShaderSource {
    pub source_url: Url,
    pub ty: ShaderType,
    pub source: String,
}

impl ShaderSource {
    pub async fn load(io: &AssetIO, shader_url: &Url) -> Result<(Self, String), AssetError> {
        log::debug!("[{}] Downloading...", shader_url.as_str());
        let source = io.download_string(&shader_url).await?;
        let ext = shader_url.extension();
        let ty = match ext {
            "vs" => Ok(ShaderType::Vertex),
            "fs" => Ok(ShaderType::Fragment),
            "cs" => Ok(ShaderType::Compute),
            _ => Err(AssetError::UnsupportedFormat(ext.to_owned())),
        }?;

        let source = ShaderSource {
            source_url: shader_url.clone(),
            ty,
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
            self.ty,
            self.source
        );

        let ty = match self.ty {
            ShaderType::Fragment => shaderc::ShaderKind::Fragment,
            ShaderType::Vertex => shaderc::ShaderKind::Vertex,
            ShaderType::Compute => shaderc::ShaderKind::Compute,
        };

        let mut compiler = shaderc::Compiler::new().unwrap();
        let options = shaderc::CompileOptions::new().unwrap();
        let compiled_artifact = compiler
            .compile_into_spirv(&self.source, ty, self.source_url.as_str(), "main", Some(&options))
            .map_err(|err| CookingError::new(self.source_url.as_str(), err))?;

        Ok(CookedShader {
            ty: self.ty,
            binary: compiled_artifact.as_binary_u8().to_owned(),
        })
    }
}
