use serde::{Deserialize, Serialize};
use shine_ecs::core::ids::SmallStringId;

pub type FrameName = SmallStringId<16>;
pub type TextureName = SmallStringId<16>;
pub type UniformName = SmallStringId<16>;

/// Semantic of the texture used in a shader
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum TextureSemantic {
    Diffuse,
    Normal,
    Frame(FrameName),
    Custom(TextureName),
}

/// Shader parameters and uniforms
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum UniformSemantic {
    Sampler(TextureSemantic),
    Texture(TextureSemantic),
    UniformBuffer(UniformName),
}

pub trait Uniform: 'static + bytemuck::Pod + bytemuck::Zeroable {
    //fn size() -> usize;
}
