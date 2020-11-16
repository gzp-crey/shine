use crate::assets::{AssetError, AssetIO, AssetId, Url};
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
    pub async fn load(io: &AssetIO, asset_base: &Url, shader_id: &AssetId) -> Result<Self, AssetError> {
        let shader_url = shader_id.to_url(asset_base)?;

        log::debug!("[{}] Downloading...", shader_url.as_str());
        let source = io.download_string(&shader_url).await?;
        let ext = shader_url.extension();
        let ty = match ext {
            "vs" => Ok(ShaderType::Vertex),
            "fs" => Ok(ShaderType::Fragment),
            "cs" => Ok(ShaderType::Compute),
            _ => Err(AssetError::UnsupportedFormat(ext.to_owned())),
        }?;

        Ok(SourceShader { ty, source })
    }
}
