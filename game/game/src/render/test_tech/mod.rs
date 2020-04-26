use crate::render::{Context, Frame, PipelineStore};
use shine_ecs::legion::systems::{
    schedule::{Schedulable, Schedule},
    SystemBuilder,
};
use wgpu;

fn render_test() -> Box<dyn Schedulable> {
    SystemBuilder::new("test_render")
        .read_resource::<Context>()
        .read_resource::<Frame>()
        .write_resource::<PipelineStore>()
        .build(move |_, _, (context, frame, _pipelines), _| {
            let mut encoder = context
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            {
                let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: frame.texture_view(),
                        resolve_target: None,
                        load_op: wgpu::LoadOp::Clear,
                        store_op: wgpu::StoreOp::Store,
                        clear_color: wgpu::Color {
                            r: 0.0,
                            g: 0.8,
                            b: 0.0,
                            a: 1.0,
                        },
                    }],
                    depth_stencil_attachment: None,
                });
            }

            frame.add_command(encoder.finish());
        })
}

pub fn create_schedule() -> Schedule {
    Schedule::builder().add_system(render_test()).flush().build()
}
