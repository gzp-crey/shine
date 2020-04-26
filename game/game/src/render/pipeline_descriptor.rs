use crate::render::Context;
use crate::wgpu;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum Primitives {
    PointList,
    LineList,
    LineStrip,
    TriangleList,
    TriangleStrip,
}

impl From<Primitives> for wgpu::PrimitiveTopology {
    fn from(pt: Primitives) -> wgpu::PrimitiveTopology {
        match pt {
            Primitives::PointList => wgpu::PrimitiveTopology::PointList,
            Primitives::LineList => wgpu::PrimitiveTopology::LineList,
            Primitives::LineStrip => wgpu::PrimitiveTopology::LineStrip,
            Primitives::TriangleList => wgpu::PrimitiveTopology::TriangleList,
            Primitives::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ValueType {
    Float,
    Float3,
    Int3,
    Float3a(u32),
    Mat2,
    Mat2a(u32),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Attribute {
    Pos2,
    Pos3,
    Norm3,
    Custom(String, ValueType),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Uniform {
    ModelView,
    Custom(String, ValueType),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VertexStage {
    pub shader: String,
    pub attributes: HashMap<u8, Attribute>,
    pub global_uniforms: HashMap<u8, Uniform>,
    pub local_uniforms: HashMap<u8, Uniform>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FragmentStage {
    pub shader: String,
    pub global_uniforms: HashMap<u8, Uniform>,
    pub local_uniforms: HashMap<u8, Uniform>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Blending {
    Replace,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PipelineDescriptor {
    pub primitives: Primitives,
    pub vertex_stage: VertexStage,
    pub fragment_stage: FragmentStage,
    color_stage: Blending,
    //depth_stencile_stage:
}

impl PipelineDescriptor {
    pub fn compile(
        &self,
        context: &Context,
        (vs, fs): (&wgpu::ShaderModule, &wgpu::ShaderModule),
    ) -> Result<wgpu::RenderPipeline, String> {
        let device = context.device();

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[],
        });

        let vertex_state = wgpu::VertexStateDescriptor {
            index_format: wgpu::IndexFormat::Uint16,
            vertex_buffers: &[],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            primitive_topology: self.primitives.into(),

            vertex_stage: wgpu::ProgrammableStageDescriptor {
                module: vs,
                entry_point: "main",
            },

            fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                module: fs,
                entry_point: "main",
            }),

            rasterization_state: None,

            color_states: &[wgpu::ColorStateDescriptor {
                format: context.swap_chain_format(),
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],

            depth_stencil_state: None,

            vertex_state,

            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Ok(pipeline)
    }
}

pub fn foo() {
    use Attribute::*;
    use Uniform::*;

    let p = PipelineDescriptor {
        primitives: Primitives::TriangleList,
        vertex_stage: VertexStage {
            shader: "pipeline/hello.vs".to_owned(),
            attributes: [
                (0, Pos3),
                (1, Norm3),
                (2, Attribute::Custom("c1".to_owned(), ValueType::Float3a(16))),
            ]
            .iter()
            .cloned()
            .collect(),
            global_uniforms: [(0, ModelView)].iter().cloned().collect(),
            local_uniforms: [(0, ModelView)].iter().cloned().collect(),
        },
        fragment_stage: FragmentStage {
            shader: "pipeline/hello.fs".to_owned(),
            global_uniforms: [(0, ModelView)].iter().cloned().collect(),
            local_uniforms: Default::default(),
        },
        color_stage: Blending::Replace,
        //depth_stencile_stage:
    };

    println!("{}", serde_json::to_string(&p).unwrap());
}
