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

use crate::{assets::AssetIO, render::Context};
use shine_ecs::core::store;

pub struct RenderStores {
    pub shaders: ShaderStore,
    pub textures: TextureStore,
    pub pipelines: PipelineStore,
    pub models: ModelStore,
}

impl RenderStores {
    pub fn new(assetio: &AssetIO) -> RenderStores {
        RenderStores {
            shaders: store::async_load::<Shader, _>(16, assetio.clone()),
            textures: store::async_load::<Texture, _>(16, assetio.clone()),
            pipelines: store::async_load::<Pipeline, _>(16, assetio.clone()),
            models: store::async_load::<Model, _>(16, assetio.clone()),
        }
    }

    pub fn update(&mut self, context: &Context) {
        self.shaders.load_and_finalize_requests((context,));
        self.textures.load_and_finalize_requests((context,));
        self.pipelines.load_and_finalize_requests((context, &self.shaders));
        self.models.load_and_finalize_requests((context,));
    }

    pub fn gc(&mut self) {
        self.models.drain_unused();
        self.pipelines.drain_unused();
        self.textures.drain_unused();
        self.shaders.drain_unused();
    }
}
use crate::{
    assets::AssetIO,
    render::{Context, Model, ModelStore, Pipeline, PipelineStore, Shader, ShaderStore, Texture, TextureStore},
};
use shine_ecs::core::store;

pub struct RenderStores {
    pub shaders: ShaderStore,
    pub textures: TextureStore,
    pub pipelines: PipelineStore,
    pub models: ModelStore,
}

impl RenderStores {
    pub fn new(assetio: &AssetIO) -> RenderStores {
        RenderStores {
            shaders: store::async_load::<Shader, _>(16, assetio.clone()),
            textures: store::async_load::<Texture, _>(16, assetio.clone()),
            pipelines: store::async_load::<Pipeline, _>(16, assetio.clone()),
            models: store::async_load::<Model, _>(16, assetio.clone()),
        }
    }

    pub fn update(&mut self, context: &Context) {
        self.shaders.load_and_finalize_requests((context,));
        self.textures.load_and_finalize_requests((context,));
        self.pipelines.load_and_finalize_requests((context, &self.shaders));
        self.models.load_and_finalize_requests((context,));
    }

    pub fn gc(&mut self) {
        self.models.drain_unused();
        self.pipelines.drain_unused();
        self.textures.drain_unused();
        self.shaders.drain_unused();
    }
}
