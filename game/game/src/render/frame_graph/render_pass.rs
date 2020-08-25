use crate::{
    assets::{PipelineStateDescriptor, UniformScope},
    render::{frame_graph::frame_target::FrameTargets, frame_graph::pass::Pass, CompiledPipeline, PipelineBindGroup},
};
use std::{
    ops::{Deref, DerefMut},
    sync::Mutex,
};

pub const DEFAULT_PASS: &str = "$";

pub struct RenderPass<'r> {
    pass: &'r Pass,
    targets: &'r FrameTargets,
    commands: &'r Mutex<Vec<wgpu::CommandBuffer>>,
    pub render_pass: wgpu::RenderPass<'r>,
}

impl<'r> RenderPass<'r> {
    pub fn new<'f: 'r>(
        pass: &'f Pass,
        targets: &'f FrameTargets,
        commands: &'f Mutex<Vec<wgpu::CommandBuffer>>,
        encoder: &'f mut wgpu::CommandEncoder,
    ) -> RenderPass<'r> {
        // Pass is given, use the attached output(s)
        let color_desc = pass
            .output
            .colors
            .iter()
            .map(|attachement| wgpu::RenderPassColorAttachmentDescriptor {
                attachment: targets.get_view(attachement.target_index),
                resolve_target: None,
                ops: attachement.operation,
            })
            .collect::<Vec<_>>();

        let depth_desc =
            pass.output
                .depth
                .as_ref()
                .map(|attachement| wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: targets.get_view(attachement.target_index),
                    depth_ops: attachement.depth_operation,
                    stencil_ops: attachement.stencil_operation,
                });

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &color_desc[..],
            depth_stencil_attachment: depth_desc,
        });

        RenderPass {
            pass,
            targets,
            commands,
            render_pass,
        }
    }

    pub fn get_pipeline_state(&self) -> &PipelineStateDescriptor {
        &self.pass.output.pipeline_state
    }

    pub fn set_pipeline(&mut self, pipeline: &'r CompiledPipeline, bindings: &'r PipelineBindGroup) {
        self.render_pass.set_pipeline(&pipeline.pipeline);
        self.render_pass
            .set_bind_group(UniformScope::Auto.bind_location(), &bindings.auto, &[]);
        self.render_pass
            .set_bind_group(UniformScope::Global.bind_location(), &bindings.global, &[]);
        self.render_pass
            .set_bind_group(UniformScope::Local.bind_location(), &bindings.local, &[]);
    }
}

impl<'r> Deref for RenderPass<'r> {
    type Target = wgpu::RenderPass<'r>;

    fn deref(&self) -> &Self::Target {
        &self.render_pass
    }
}

impl<'r> DerefMut for RenderPass<'r> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.render_pass
    }
}
