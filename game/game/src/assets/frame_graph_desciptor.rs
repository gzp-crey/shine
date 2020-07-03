use crate::assets::AssetError;
use crate::assets::{RenderTargetDescriptor, SamplerDescriptor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FramePassMethod {
    /// Copy source into target using the given pipeline
    FullScreenQuad(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FramePassDescriptor {
    pub input: HashMap<String, SamplerDescriptor>,
    pub output: Vec<String>,
    pub method: FramePassMethod,
}

/*impl FramePassDescriptor {
    pub fn to_frame_pass(&self, device: &wgpu::Device) -> Result<FramePass, AssetError> {
        //self.input.map(|k,v| )

    }
}*/

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameGraphDescriptor {
    pub targets: HashMap<String, RenderTargetDescriptor>,
    pub passes: HashMap<String, FramePassDescriptor>,
}

/*impl FrameGraphDescriptor {
    pub fn to_frame_graph(&self, device: &wgpu::Device) -> Result<FramePass, AssetError> {
        self.input.iter().map(|k, v| (k, v.to_texture(device)));
    }
}*/

pub struct FramePass {
    pub input: HashMap<String, wgpu::Sampler>,
    pub output: Vec<String>,
    pub method: FramePassMethod,
}

pub struct FrameGraphBuffer {
    pub textures: HashMap<String, (wgpu::Texture, wgpu::TextureView)>,
    //pub passes: HashMap<String, FramePass>,
}
