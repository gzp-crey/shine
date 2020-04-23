use crate::utils::url::Url;
use crate::{
    render::{Context, PipelineDescriptor, ReadShaderStore, ShaderIndex, ShaderStore},
    utils, wgpu, GameError,
};
use futures::future::FutureExt;
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::pin::Pin;

pub struct Dependecy {
    vertex_shader: ShaderIndex,
    //fragment_shader: ShaderIndex,
}

impl Dependecy {
    fn from_descriptor<'l, 's>(
        load_context: &LoadContext<'l, Pipeline>,
        descriptor: &PipelineDescriptor,
        mut shaders: ReadShaderStore<'s>,
    ) -> Dependecy {
        let vertex_shader = {
            let id = shaders.named_get_or_add_blocking(&descriptor.vertex_stage.shader);
            shaders[&id].is_ready(load_context, PipelineLoadResult::VertexShaderReady);
            id
        };

        Dependecy { vertex_shader }
        //vertex_shader
    }
}

pub struct Compiled {}

pub enum Pipeline {
    None,
    Preparing(PipelineDescriptor, Dependecy),
    Compiled(Compiled),
    Error(String),
}

pub enum PipelineLoadResult {
    Error(String),
    Descriptor(PipelineDescriptor),
    VertexShaderReady,
}

impl Data for Pipeline {
    type Key = String;
    type LoadRequest = String;
    type LoadResponse = PipelineLoadResult;
    type UpdateContext = (Context, ShaderStore);

    fn on_load<'a>(
        &mut self,
        load_context: LoadContext<'a, Pipeline>,
        (context, shaders): &Self::UpdateContext,
        load_response: PipelineLoadResult,
    ) -> Option<String> {
        match load_response {
            PipelineLoadResult::Error(err) => {
                *self = Pipeline::Error(err);
            }
            PipelineLoadResult::Descriptor(descriptor) => {
                //shaders[vertex_shader].is_ready(lc, VertexShaderReady)
                let dependency = Dependecy::from_descriptor(&load_context, &descriptor, shaders.read());
                *self = Pipeline::Preparing(descriptor, dependency);
            }
            _ => {}
        }
        None
    }
}

impl FromKey for Pipeline {
    fn from_key(key: &String) -> (Self, Option<String>) {
        (Pipeline::None, Some(key.to_owned()))
    }
}

pub struct PipelineLoader {
    base_url: Url,
}

impl PipelineLoader {
    pub fn new(base_url: &str) -> Result<PipelineLoader, GameError> {
        let base_url = Url::parse(base_url)
            .map_err(|err| GameError::Config(format!("Failed to parse base url for pipeline: {:?}", err)))?;

        Ok(PipelineLoader { base_url })
    }

    async fn load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Pipeline>,
        source_id: String,
    ) -> Option<PipelineLoadResult> {
        if cancellation_token.is_canceled() {
            return None;
        }

        let url = match self.base_url.join(&source_id) {
            Err(err) => {
                let err = format!("Invalid pipeline url ({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResult::Error(err));
            }
            Ok(url) => url,
        };

        let data = match utils::assets::download_string(&url).await {
            Err(err) => {
                let err = format!("Failed to get pipeline({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResult::Error(err));
            }
            Ok(data) => data,
        };
        log::trace!("pipeline [{}]: {}", source_id, data);

        Some(PipelineLoadResult::Error("not implemented".to_owned()))
    }
}

impl DataLoader<Pipeline> for PipelineLoader {
    fn load<'a>(
        &'a mut self,
        pipeline_id: String,
        cancellation_token: CancellationToken<Pipeline>,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<PipelineLoadResult>> + Send + 'a>> {
        self.load_from_url(cancellation_token, pipeline_id).boxed()
    }
}

pub type PipelineStore = Store<Pipeline>;
