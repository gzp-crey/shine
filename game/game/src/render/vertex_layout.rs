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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VertexAttribute {
    Position(wgpu::BufferAddress, wgpu::VertexFormat),
    Color(wgpu::BufferAddress, wgpu::VertexFormat),
    Custom(String, wgpu::BufferAddress, wgpu::VertexFormat),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct VertexBufferLayout {
    pub stride: wgpu::BufferAddress,
    pub attributes: Vec<Vec<VertexAttribute>>,
}

impl VertexBufferLayout {
    pub fn from_id(id: &VertexTypeId) -> VertexBufferLayout {
        bincode::deserialize(&id.0).unwrap()
    }

    pub fn to_id(&self) -> VertexTypeId {
        VertexTypeId(bincode::serialize(self).unwrap())
    }
}

pub trait Vertex: 'static + bytemuck::Pod + bytemuck::Zeroable {
    fn buffer_layout() -> VertexBufferLayout;

    fn id() -> VertexTypeId
    where
        Self: Sized,
    {
        Self::buffer_layout().to_id()
    }
}

pub mod vertex {
    use super::*;

    /// Vertex without atributes.
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
        pub position: (f32, f32, f32),
        pub color: (f32, f32, f32),
    }

    unsafe impl bytemuck::Pod for Pos3fCol3f {}
    unsafe impl bytemuck::Zeroable for Pos3fCol3f {}

    impl Vertex for Pos3fCol3f {
        #[allow(clippy::fn_to_numeric_cast)]
        fn buffer_layout() -> VertexBufferLayout {
            VertexBufferLayout {
                stride: mem::size_of::<Self> as wgpu::BufferAddress,
                attributes: vec![vec![
                    VertexAttribute::Position(0, wgpu::VertexFormat::Float3),
                    VertexAttribute::Color(24, wgpu::VertexFormat::Float3),
                ]],
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct Pos3fCol4f {
        pub position: (f32, f32, f32),
        pub color: (f32, f32, f32, f32),
    }

    unsafe impl bytemuck::Pod for Pos3fCol4f {}
    unsafe impl bytemuck::Zeroable for Pos3fCol4f {}

    impl Vertex for Pos3fCol4f {
        #[allow(clippy::fn_to_numeric_cast)]
        fn buffer_layout() -> VertexBufferLayout {
            VertexBufferLayout {
                stride: mem::size_of::<Self> as wgpu::BufferAddress,
                attributes: vec![vec![
                    VertexAttribute::Position(0, wgpu::VertexFormat::Float3),
                    VertexAttribute::Color(24, wgpu::VertexFormat::Float4),
                ]],
            }
        }
    }
}
