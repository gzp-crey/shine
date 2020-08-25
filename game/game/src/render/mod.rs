mod surface;
pub use self::surface::*;
mod context;
pub use self::context::*;
mod plugin;
pub use self::plugin::*;

mod compile;
pub use self::compile::*;
mod compiled_shader;
pub use self::compiled_shader::*;
mod compiled_texture;
pub use self::compiled_texture::*;
mod compiled_render_target;
pub use self::compiled_render_target::*;
mod compiled_pipeline;
pub use self::compiled_pipeline::*;
mod compiled_model;
pub use self::compiled_model::*;

mod shader;
pub use self::shader::{Shader, ShaderDependency, ShaderError, ShaderIndex, ShaderKey, ShaderStore, ShaderStoreRead};
mod texture;
pub use self::texture::{
    Texture, TextureDependency, TextureError, TextureIndex, TextureKey, TextureStore, TextureStoreRead,
};
mod pipeline;
pub use self::pipeline::{
    Pipeline, PipelineDependency, PipelineError, PipelineIndex, PipelineKey, PipelineStore, PipelineStoreRead,
};
mod model;
pub use self::model::{Model, ModelError, ModelIndex, ModelKey, ModelStore, ModelStoreRead};
mod frame_graph;
pub use self::frame_graph::*;

pub mod systems {
    //pub use super::frame_graph::systems::*;
    pub use super::model::systems::*;
    pub use super::pipeline::systems::*;
    pub use super::shader::systems::*;
    pub use super::texture::systems::*;
}
