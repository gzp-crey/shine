use crate::assets::VertexSemantic;
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

impl VertexTypeId {
    pub fn from_layout(layout: &VertexBufferLayout) -> Self {
        VertexTypeId(bincode::serialize(&vec![layout]).unwrap())
    }

    pub fn from_layouts(layouts: &Vec<VertexBufferLayout>) -> Self {
        VertexTypeId(bincode::serialize(layouts).unwrap())
    }

    pub fn to_layout(&self) -> Vec<VertexBufferLayout> {
        bincode::deserialize(&self.0).unwrap()
    }
}

pub trait IntoVertexTypeId {
    fn into_id() -> VertexTypeId;
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct VertexBufferLayout {
    pub stride: wgpu::BufferAddress,
    pub attributes: Vec<VertexAttribute>,
}

pub trait Vertex: 'static + bytemuck::Pod + bytemuck::Zeroable {
    fn buffer_layout() -> VertexBufferLayout;
}

impl<T> IntoVertexTypeId for T
where
    T: Vertex,
{
    fn into_id() -> VertexTypeId {
        VertexTypeId::from_layout(&Self::buffer_layout())
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
                    VertexAttribute(Position, 0, Float3),
                    VertexAttribute(Color(0), 12, Float3),
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
                    VertexAttribute(Position, 0, Float3),
                    VertexAttribute(Color(0), 12, Float4),
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
                    VertexAttribute(Position, 0, Float3),
                    VertexAttribute(TexCoord(0), 8, Float2),
                ],
            }
        }
    }
}
