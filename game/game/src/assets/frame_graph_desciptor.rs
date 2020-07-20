use crate::assets::AssetError;
use crate::assets::{RenderTargetDescriptor, SamplerDescriptor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FramePassMethod {
    /// Execute the given logic
    Scene(String),

    /// Copy source into target using the given pipeline
    FullScreenQuad(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FramePassDescriptor {
    pub input: HashMap<String, SamplerDescriptor>,
    pub output: Vec<String>,
    pub method: FramePassMethod,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameGraphDescriptor {
    pub targets: HashMap<String, RenderTargetDescriptor>,
    pub passes: HashMap<String, FramePassDescriptor>,
}
