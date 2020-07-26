use crate::assets::SamplerDescriptor;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RenderTargetSize {
    /// Size matching the frame output
    Matching,
    /// Size propotional to the render target
    Propotional(f32, f32),
    /// Fixed sized
    Fixed(u32, u32),
}

/// Render target descriptor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderTargetDescriptor {
    pub format: wgpu::TextureFormat,
    pub size: RenderTargetSize,
}

impl RenderTargetDescriptor {
    pub fn get_target_size(&self, frame_size: (u32, u32)) -> (u32, u32) {
        match &self.size {
            RenderTargetSize::Matching => frame_size,
            RenderTargetSize::Fixed(w, h) => (*w, *h),
            RenderTargetSize::Propotional(sw, sh) => {
                let w = ((frame_size.0 as f32) * sw).clamp(4., 65536.) as u32;
                let h = ((frame_size.1 as f32) * sh).clamp(4., 65536.) as u32;
                (w, h)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorAttachementDescriptor {
    texture: String,
    operation: wgpu::Operations<wgpu::Color>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepthAttachementDescriptor {
    texture: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderAttachementDescriptor {
    color: Vec<ColorAttachementDescriptor>,
    depth: Option<DepthAttachementDescriptor>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderSourceDescriptor {
    texture: String,
    sampler: SamplerDescriptor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FramePassMethod {
    /// Execute the given logic
    Scene(String),

    /// Copy source into target using the given pipeline
    FullScreenQuad(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FramePassDescriptor {
    pub input: Vec<RenderSourceDescriptor>,
    pub output: Vec<RenderAttachementDescriptor>,
    //pub method: FramePassMethod,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameGraphDescriptor {
    pub targets: HashMap<String, RenderTargetDescriptor>,
    pub passes: HashMap<String, FramePassDescriptor>,
}
