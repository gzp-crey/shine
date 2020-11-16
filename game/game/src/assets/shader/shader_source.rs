use crate::assets::{io::HashableContent, AssetError, AssetIO, Url};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SourceShader {
    pub ty: ShaderType,
    pub source: String,
}

impl SourceShader {
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

        let source = SourceShader { ty, source };
        let source_hash = source.source.content_hash();

        Ok((source, source_hash))
    }
}
