use crate::assets::AssetError;
use crate::assets::TextureDescriptor;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pass {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameGraphDescriptor {
    pub textures: Vec<(String, TextureDescriptor)>,
    pub passes: Vec<String>,
}
