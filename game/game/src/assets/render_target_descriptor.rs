use serde::{Deserialize, Serialize};
use shine_ecs::resources::ResourceTag;

pub type TextureTargetTag = ResourceTag;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorAttachementDescriptor {
    /// The tag of the TextureTarget resource to use or None to render to the FrameTarget
    pub target: Option<TextureTargetTag>,
    pub operation: wgpu::Operations<wgpu::Color>,
    pub alpha_blend: wgpu::BlendDescriptor,
    pub color_blend: wgpu::BlendDescriptor,
    pub write_mask: wgpu::ColorWrite,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StencilAttachementOperation {
    pub operation: Option<wgpu::Operations<u32>>,
    pub front: wgpu::StencilStateFaceDescriptor,
    pub back: wgpu::StencilStateFaceDescriptor,
    pub read_mask: u32,
    pub write_mask: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepthAttachementOperation {
    pub operation: Option<wgpu::Operations<f32>>,
    pub write_enabled: bool,
    pub compare: wgpu::CompareFunction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepthAttachementDescriptor {
    pub target: TextureTargetTag,
    pub depth_operation: DepthAttachementOperation,
    pub stencil_operation: StencilAttachementOperation,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct RenderTargetDescriptor {
    pub colors: Vec<ColorAttachementDescriptor>,
    pub depth: Option<DepthAttachementDescriptor>,
}
