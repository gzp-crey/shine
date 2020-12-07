use crate::assets::AssetError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

impl ShaderType {
    pub fn from_extension(s: &str) -> Result<Self, AssetError> {
        match s {
            "fs" | "fs_spv" => Ok(ShaderType::Fragment),
            "vs" | "vs_spv" => Ok(ShaderType::Vertex),
            "cs" | "cs_spv" => Ok(ShaderType::Compute),
            _ => Err(AssetError::UnsupportedFormat(s.to_owned())),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct CookedShader {
    pub shader_type: ShaderType,
    pub binary: Vec<u8>,
}
