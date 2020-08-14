use crate::assets::{AssetError, RenderTargetDescriptor, SamplerDescriptor};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const FRAME_TARGET_NAME: &str = "FRAME";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorAttachementDescriptor {
    pub target: String,
    pub operation: wgpu::Operations<wgpu::Color>,
    pub alpha_blend: wgpu::BlendDescriptor,
    pub color_blend: wgpu::BlendDescriptor,
    pub write_mask: wgpu::ColorWrite,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DepthAttachementOperation {
    pub operation: Option<wgpu::Operations<f32>>,
    pub write_enabled: bool,
    pub compare: wgpu::CompareFunction,
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
pub struct DepthAttachementDescriptor {
    pub target: String,
    pub depth_operation: DepthAttachementOperation,
    pub stencil_operation: StencilAttachementOperation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderAttachementDescriptor {
    pub colors: Vec<ColorAttachementDescriptor>,
    pub depth: Option<DepthAttachementDescriptor>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderSourceDescriptor {
    pub target: String,
    pub sampler: SamplerDescriptor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FramePassDescriptor {
    pub inputs: Vec<RenderSourceDescriptor>,
    pub output: RenderAttachementDescriptor,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameGraphDescriptor {
    pub targets: HashMap<String, RenderTargetDescriptor>,
    pub passes: HashMap<String, FramePassDescriptor>,
}

impl FrameGraphDescriptor {
    pub fn is_target_sampled(&self, target: &str) -> bool {
        for pass in self.passes.values() {
            if pass.inputs.iter().any(|x| x.target == target) {
                return true;
            }
        }
        false
    }

    /// Check if target - pass references are correct
    pub fn check_target_references(&self) -> Result<(), AssetError> {
        for (pass_name, pass) in self.passes.iter() {
            for input in pass.inputs.iter() {
                if self.targets.get(&input.target).is_none() {
                    return Err(AssetError::Content(format!(
                        "Pass {} references an unknown input: {}",
                        pass_name, input.target
                    )));
                }
            }

            if let Some(depth) = &pass.output.depth {
                if self.targets.get(&depth.target).is_none()
                /* && depth.target != FRAME_TARGET_NAME*/
                {
                    return Err(AssetError::Content(format!(
                        "Pass {} references an unknown depth target: {}",
                        pass_name, depth.target
                    )));
                }
            }

            for color in pass.output.colors.iter() {
                if self.targets.get(&color.target).is_none() && color.target != FRAME_TARGET_NAME {
                    return Err(AssetError::Content(format!(
                        "Pass {} references an unknown color target: {}",
                        pass_name, color.target
                    )));
                }
            }
        }

        Ok(())
    }
}
