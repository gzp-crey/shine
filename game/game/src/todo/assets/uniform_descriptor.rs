use serde::{Deserialize, Serialize};
use std::mem;

/// Semantic of the texture used in a shader
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum TextureSemantic {
    Diffuse,
    Normal,
    Frame(String),
    Custom(String),
}

/// Semantic of the uniform used in a shader
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum UniformSemantic {
    ViewProj,
    Raw(String, usize),
}

impl UniformSemantic {
    pub fn size(&self) -> usize {
        match self {
            UniformSemantic::ViewProj => mem::size_of::<uniform::ViewProj>(),
            UniformSemantic::Raw(_, s) => *s,
        }
    }
}

/// Shader parameters and uniforms
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum Uniform {
    Sampler(TextureSemantic),
    Texture(TextureSemantic),
    UniformBuffer(UniformSemantic),
}

pub mod uniform {
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct ViewProj {
        pub mx: [f32; 16],
    }

    unsafe impl bytemuck::Pod for ViewProj {}
    unsafe impl bytemuck::Zeroable for ViewProj {}
}
