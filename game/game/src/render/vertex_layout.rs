use crate::wgpu;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::{fmt, mem};

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct VertexTypeId(Vec<u8>);

impl fmt::Debug for VertexTypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        f.debug_tuple("VertexTypeId").field(&hasher.finish()).finish()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VertexAttribute {
    Position(wgpu::BufferAddress, wgpu::VertexFormat),
    Color(wgpu::BufferAddress, wgpu::VertexFormat),
    Custom(String, wgpu::BufferAddress, wgpu::VertexFormat),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VertexBufferLayout {
    pub stride: wgpu::BufferAddress,
    pub attributes: Vec<VertexAttribute>,
}

impl VertexBufferLayout {
    pub fn from_id(id: &VertexTypeId) -> VertexBufferLayout {
        bincode::deserialize(&id.0).unwrap()
    }

    pub fn into_id(&self) -> VertexTypeId {
        VertexTypeId(bincode::serialize(self).unwrap())
    }
}

pub trait Vertex: 'static {
    fn buffer_layout() -> VertexBufferLayout;

    fn id() -> VertexTypeId
    where
        Self: Sized,
    {
        Self::buffer_layout().into_id()
    }
}

/// Vertex without atributes.
#[repr(C)]
pub struct VertexNull {}

impl Vertex for VertexNull {
    fn buffer_layout() -> VertexBufferLayout {
        VertexBufferLayout {
            stride: 0,
            attributes: Vec::new(),
        }
    }
}

#[repr(C)]
pub struct VertexP2C3 {
    pos: (f32, f32),
    color: (f32, f32, f32),
}

impl Vertex for VertexP2C3 {
    fn buffer_layout() -> VertexBufferLayout {
        VertexBufferLayout {
            stride: mem::size_of::<Self> as wgpu::BufferAddress,
            attributes: vec![
                VertexAttribute::Position(0, wgpu::VertexFormat::Float2),
                VertexAttribute::Color(16, wgpu::VertexFormat::Float3),
            ],
        }
    }
}
