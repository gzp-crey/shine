use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum VertexSemantic {
    Position,
    Color(u8),
    TexCoord(u8),
    Normal,
    Tangent,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct VertexAttribute(VertexSemantic, wgpu::BufferAddress, wgpu::VertexFormat);

impl VertexAttribute {
    pub fn new(semantic: VertexSemantic, offset: wgpu::BufferAddress, format: wgpu::VertexFormat) -> VertexAttribute {
        VertexAttribute(semantic, offset, format)
    }

    pub fn semantic(&self) -> &VertexSemantic {
        &self.0
    }

    pub fn offset(&self) -> wgpu::BufferAddress {
        self.1
    }

    pub fn format(&self) -> wgpu::VertexFormat {
        self.2
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VertexBufferLayout {
    pub stride: wgpu::BufferAddress,
    pub attributes: Vec<VertexAttribute>,
}

pub type VertexBufferLayouts = Vec<VertexBufferLayout>;

pub trait Vertex: 'static + bytemuck::Pod + bytemuck::Zeroable {
    fn buffer_layout() -> VertexBufferLayout;
}
