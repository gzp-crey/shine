mod surface;
pub use self::surface::*;
mod context;
pub use self::context::*;
mod system;
pub use self::system::*;
mod frame;
pub use self::frame::*;
mod shader;
pub use self::shader::{Shader, ShaderDependency, ShaderIndex, ShaderKey, ShaderStore, ShaderStoreRead};
mod texture;
pub use self::texture::{Texture, TextureIndex, TextureKey, TextureNamedId, TextureStore, TextureStoreRead};
mod pipeline;
pub use self::pipeline::{Pipeline, PipelineIndex, PipelineKey, PipelineNamedId, PipelineStore, PipelineStoreRead};
//mod frame_graph;
//pub use self::frame_graph::{
//    FrameGraph, FrameGraphId, FrameGraphIndex, FrameGraphKey, FrameGraphLoader, FrameGraphStore, FrameGraphStoreRead,
//};
mod model;
pub use self::model::{Model, ModelIndex, ModelKey, ModelStore, ModelStoreRead};

pub mod systems {
    //pub use super::frame_graph::systems::*;
    pub use super::model::systems::*;
    pub use super::pipeline::systems::*;
    pub use super::shader::systems::*;
    pub use super::texture::systems::*;
}
