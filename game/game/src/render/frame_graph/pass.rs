use crate::{
    assets::{FramePassDescriptor, PipelineStateDescriptor, RenderAttachementDescriptor, RenderSourceDescriptor},
    render::{frame_graph::frame_target::FrameTargets, Compile, FrameGraphError},
};

pub struct PassDepthOutput {
    pub target_index: usize,
    pub depth_operation: Option<wgpu::Operations<f32>>,
    pub stencil_operation: Option<wgpu::Operations<u32>>,
}

pub struct PassColorOutput {
    pub target_index: usize,
    pub operation: wgpu::Operations<wgpu::Color>,
}

pub struct PassOutput {
    pub depth: Option<PassDepthOutput>,
    pub colors: Vec<PassColorOutput>,
    pub pipeline_state: PipelineStateDescriptor,
}

impl PassOutput {
    fn create(targets: &FrameTargets, descriptor: &RenderAttachementDescriptor) -> Result<PassOutput, FrameGraphError> {
        let (output_depth, stage_depth) = match &descriptor.depth {
            Some(depth) => {
                let depth_output = PassDepthOutput {
                    target_index: targets.find_target_index(&depth.target).ok_or(FrameGraphError)?,
                    depth_operation: depth.depth_operation.operation.clone(),
                    stencil_operation: depth.stencil_operation.operation.clone(),
                };
                let stage_depth = wgpu::DepthStencilStateDescriptor {
                    format: targets.get_texture_format(depth_output.target_index),
                    depth_write_enabled: depth.depth_operation.write_enabled,
                    depth_compare: depth.depth_operation.compare,
                    stencil: wgpu::StencilStateDescriptor {
                        front: depth.stencil_operation.front.clone(),
                        back: depth.stencil_operation.back.clone(),
                        read_mask: depth.stencil_operation.read_mask,
                        write_mask: depth.stencil_operation.write_mask,
                    },
                };
                (Some(depth_output), Some(stage_depth))
            }
            None => (None, None),
        };

        let mut stage_colors = Vec::with_capacity(descriptor.colors.len());
        let mut output_colors = Vec::with_capacity(descriptor.colors.len());
        for color in descriptor.colors.iter() {
            let output_color = PassColorOutput {
                target_index: targets.find_target_index(&color.target).ok_or(FrameGraphError)?,
                operation: color.operation,
            };
            let stage_color = wgpu::ColorStateDescriptor {
                format: targets.get_texture_format(output_color.target_index),
                alpha_blend: color.alpha_blend.clone(),
                color_blend: color.color_blend.clone(),
                write_mask: color.write_mask,
            };
            output_colors.push(output_color);
            stage_colors.push(stage_color);
        }

        Ok(PassOutput {
            depth: output_depth,
            colors: output_colors,
            pipeline_state: PipelineStateDescriptor {
                depth_state: stage_depth,
                color_states: stage_colors,
            },
        })
    }
}

/// Pass render inputs indexing the render targets
pub struct PassInput {
    texture_index: usize,
    sampler: wgpu::Sampler,
}

impl PassInput {
    fn create(
        device: &wgpu::Device,
        targets: &FrameTargets,
        descriptor: &RenderSourceDescriptor,
    ) -> Result<PassInput, FrameGraphError> {
        Ok(PassInput {
            texture_index: targets.find_target_index(&descriptor.target).ok_or(FrameGraphError)?,
            sampler: descriptor.sampler.compile(device, ()),
        })
    }
}

pub struct Pass {
    pub name: String,
    pub inputs: Vec<PassInput>,
    pub output: PassOutput,
}

impl Pass {
    pub fn create(
        device: &wgpu::Device,
        targets: &FrameTargets,
        name: &str,
        descriptor: &FramePassDescriptor,
    ) -> Result<Pass, FrameGraphError> {
        let inputs = descriptor
            .inputs
            .iter()
            .map(|input| PassInput::create(device, targets, input))
            .collect::<Result<Vec<_>, _>>()?;
        let output = PassOutput::create(targets, &descriptor.output)?;
        Ok(Pass {
            name: name.to_owned(),
            inputs,
            output,
        })
    }
}
