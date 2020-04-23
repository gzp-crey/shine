use crate::wgpu;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
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
    pub primitive_topology: Primitives,
    pub vertex_stage: VertexStage,
    pub fragment_stage: FragmentStage,
    color_stage: Blending,
    //depth_stencile_stage:
}

pub fn foo() {
    use Attribute::*;
    use Uniform::*;

    let p = PipelineDescriptor {
        primitive_topology: Primitives::TriangleList,
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
