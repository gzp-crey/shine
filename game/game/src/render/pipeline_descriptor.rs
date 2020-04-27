use crate::render::{Context, VertexBufferLayout};
use crate::wgpu;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum PipelineAttribute {
    Position(wgpu::VertexFormat),
    Color(wgpu::VertexFormat),
    Normal(wgpu::VertexFormat),
    Named(String, wgpu::VertexFormat),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Uniform {
    ModelView,
    //Named(String, wgpu::VertexFormat),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VertexStage {
    pub shader: String,
    pub attributes: HashMap<u8, PipelineAttribute>,
    pub global_uniforms: HashMap<u8, Uniform>,
    pub local_uniforms: HashMap<u8, Uniform>,
}

impl VertexStage {
    pub fn check_vertex_layout(&self, vertex_layout: &VertexBufferLayout) -> Result<(), String> {
        if self.attributes.len() > vertex_layout.attributes.len() {
            return Err("Missing vertex attributes".to_string());
        }
        //todo: more checks
        Ok(())
    }
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
    pub primitive_topology: wgpu::PrimitiveTopology,
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
            primitive_topology: self.primitive_topology,

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
    use PipelineAttribute::*;
    use Uniform::*;

    let p = PipelineDescriptor {
        primitive_topology: wgpu::PrimitiveTopology::TriangleList,
        vertex_stage: VertexStage {
            shader: "pipeline/hello.vs".to_owned(),
            attributes: [
                (0, Position(wgpu::VertexFormat::Float2)),
                (1, Normal(wgpu::VertexFormat::Float3)),
                (
                    2,
                    PipelineAttribute::Named("c1".to_owned(), wgpu::VertexFormat::Ushort2Norm),
                ),
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
