use crate::assets::{Vertex, VertexAttribute, VertexBufferLayout, VertexSemantic};
use std::mem;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Null {}

unsafe impl bytemuck::Pod for Null {}
unsafe impl bytemuck::Zeroable for Null {}

impl Vertex for Null {
    fn buffer_layout() -> VertexBufferLayout {
        VertexBufferLayout {
            stride: 0,
            attributes: Vec::new(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Pos3fCol3f {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

unsafe impl bytemuck::Pod for Pos3fCol3f {}
unsafe impl bytemuck::Zeroable for Pos3fCol3f {}

impl Vertex for Pos3fCol3f {
    #[allow(clippy::fn_to_numeric_cast)]
    fn buffer_layout() -> VertexBufferLayout {
        use wgpu::VertexFormat::*;
        use VertexSemantic::*;
        VertexBufferLayout {
            stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            attributes: vec![
                VertexAttribute::new(Position, 0, Float3),
                VertexAttribute::new(Color(0), 12, Float3),
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Pos3fCol4f {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

unsafe impl bytemuck::Pod for Pos3fCol4f {}
unsafe impl bytemuck::Zeroable for Pos3fCol4f {}

impl Vertex for Pos3fCol4f {
    #[allow(clippy::fn_to_numeric_cast)]
    fn buffer_layout() -> VertexBufferLayout {
        use wgpu::VertexFormat::*;
        use VertexSemantic::*;
        VertexBufferLayout {
            stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            attributes: vec![
                VertexAttribute::new(Position, 0, Float3),
                VertexAttribute::new(Color(0), 12, Float4),
            ],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Pos3fTex2f {
    pub position: [f32; 3],
    pub texcoord: [f32; 2],
}

unsafe impl bytemuck::Pod for Pos3fTex2f {}
unsafe impl bytemuck::Zeroable for Pos3fTex2f {}

impl Vertex for Pos3fTex2f {
    #[allow(clippy::fn_to_numeric_cast)]
    fn buffer_layout() -> VertexBufferLayout {
        use wgpu::VertexFormat::*;
        use VertexSemantic::*;
        VertexBufferLayout {
            stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            attributes: vec![
                VertexAttribute::new(Position, 0, Float3),
                VertexAttribute::new(TexCoord(0), 12, Float2),
            ],
        }
    }
}
