use crate::{
    assets::{PipelineStateDescriptor, UniformScope},
    render::{frame_graph::frame_target::FrameTargets, CompiledPipeline, PipelineBindGroup},
};
use std::ops::{Deref, DerefMut};

enum RenderPassTarget<'a>
{
    FrameOutput(&'a FrameOutput)
}

pub struct RenderPass<'r> {
    target: RenderPassTarget<'r>,
    pipeline_state: &'r PipelineStateDescriptor,
    render_pass: wgpu::RenderPass<'r>,
}

impl<'r> RenderPass<'r> {
    pub fn begin_hack<'r, 'f: 'r, 'e: 'f>(
        encoder: &'e mut wgpu::CommandEncoder,
        frame_output: &'f FrameOutput,
    ) -> Result<RenderPass<'r>, RenderError> {
        let color_desc = [wgpu::RenderPassColorAttachmentDescriptor {
            attachment: frame_output.get_view(),
            resolve_target: None,
            ops: color.operation,
            }];

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &color_desc,
            depth_stencil_attachment: None,
        });

        Ok(RenderPass{
            render_pass
        })
    }

    /*pub fn new<'q, 'f: 'q>(
        render_pass: wgpu::RenderPass<'q>,
        pipeline_state: &'f PipelineStateDescriptor,
        targets: &'f FrameTargets,
    ) -> RenderPass<'q> {
        RenderPass {
            targets,
            pipeline_state,
            render_pass,
        }
    }*/

    pub fn get_pipeline_state(&self) -> &PipelineStateDescriptor {
        &self.pipeline_state
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
