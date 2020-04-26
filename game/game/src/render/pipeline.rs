use crate::utils::url::Url;
use crate::{
    render::{Context, PipelineDescriptor, ShaderDependency, ShaderStore, ShaderStoreRead, ShaderType},
    utils, wgpu, GameError,
};
use futures::future::FutureExt;
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, DataUpdater, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::pin::Pin;

pub struct Dependecy {
    descriptor: PipelineDescriptor,
    vertex_shader: ShaderDependency,
    fragment_shader: ShaderDependency,
}

impl Dependecy {
    fn from_descriptor(
        load_context: &LoadContext<'_, Pipeline>,
        descriptor: PipelineDescriptor,
        shaders: &mut ShaderStoreRead<'_>,
    ) -> Dependecy {
        let vertex_shader = ShaderDependency::new(
            shaders,
            &descriptor.vertex_stage.shader,
            ShaderType::Vertex,
            load_context,
            PipelineLoadResponse::ShaderReady(ShaderType::Vertex),
        );
        let fragment_shader = ShaderDependency::new(
            shaders,
            &descriptor.fragment_stage.shader,
            ShaderType::Fragment,
            load_context,
            PipelineLoadResponse::ShaderReady(ShaderType::Fragment),
        );

        Dependecy {
            descriptor,
            vertex_shader,
            fragment_shader,
        }
    }

    fn with_updated_shader_dependency(self, shader_type: ShaderType, shaders: &mut ShaderStoreRead<'_>) -> Self {
        match shader_type {
            ShaderType::Vertex => Dependecy {
                vertex_shader: self.vertex_shader.update(shaders),
                ..self
            },
            ShaderType::Fragment => Dependecy {
                fragment_shader: self.fragment_shader.update(shaders),
                ..self
            },
            _ => unreachable!(),
        }
    }

    fn into_pipeline(
        self,
        load_context: &LoadContext<'_, Pipeline>,
        context: &Context,
        shaders: &mut ShaderStoreRead<'_>,
        listeners: LoadListeners,
    ) -> Pipeline {
        match (&self.vertex_shader, &self.fragment_shader) {
            (ShaderDependency::Failed(err), _) => {
                listeners.notify_all();
                Pipeline::Error(format!("Vertex shader error: {}", err))
            }
            (_, ShaderDependency::Failed(err)) => {
                listeners.notify_all();
                Pipeline::Error(format!("Fragment shader error: {}", err))
            }
            (ShaderDependency::Pending(_, _), _) => Pipeline::WaitingDependency(self, listeners),
            (_, ShaderDependency::Pending(_, _)) => Pipeline::WaitingDependency(self, listeners),
            (ShaderDependency::Completed(vs), ShaderDependency::Completed(fs)) => {
                log::debug!("Pipeline compilation completed [{}]", load_context);
                listeners.notify_all();
                let vs = shaders[&vs].shadere_module().unwrap();
                let fs = shaders[&fs].shadere_module().unwrap();
                match self.descriptor.compile(context, (vs, fs)) {
                    Ok(pipeline) => Pipeline::Compiled(pipeline),
                    Err(err) => Pipeline::Error(err),
                }
            }
        }
    }
}

pub enum Pipeline {
    Pending(LoadListeners),
    WaitingDependency(Dependecy, LoadListeners),
    Compiled(wgpu::RenderPipeline),
    Error(String),
    None,
}

impl Pipeline {
    fn on_load(
        &mut self,
        load_context: LoadContext<'_, Pipeline>,
        context: &Context,
        shaders: &mut ShaderStoreRead<'_>,
        load_response: PipelineLoadResponse,
    ) -> Option<String> {
        *self = match (std::mem::replace(self, Pipeline::None), load_response) {
            (Pipeline::Pending(listeners), PipelineLoadResponse::Error(err)) => {
                log::debug!("Pipeline compilation failed [{}]: {}", load_context, err);
                listeners.notify_all();
                Pipeline::Error(err)
            }

            (Pipeline::Pending(listeners), PipelineLoadResponse::Descriptor(descriptor)) => {
                let dependency = Dependecy::from_descriptor(&load_context, descriptor, shaders);
                dependency.into_pipeline(&load_context, context, shaders, listeners)
            }

            (Pipeline::WaitingDependency(dependency, listeners), PipelineLoadResponse::ShaderReady(shader_type)) => {
                dependency
                    .with_updated_shader_dependency(shader_type, shaders)
                    .into_pipeline(&load_context, context, shaders, listeners)
            }

            (err @ Pipeline::Error(_), PipelineLoadResponse::ShaderReady(_)) => err,

            (Pipeline::WaitingDependency(_, _), _) => unreachable!(),
            (Pipeline::Pending(_), _) => unreachable!(),
            (Pipeline::Compiled(_), _) => unreachable!(),
            (Pipeline::Error(_), _) => unreachable!(),
            (Pipeline::None, _) => unreachable!(),
        };

        None
    }
}

impl Data for Pipeline {
    type Key = String;
    type LoadRequest = PipelineLoadRequest;
    type LoadResponse = PipelineLoadResponse;
}

impl FromKey for Pipeline {
    fn from_key(key: &String) -> (Self, Option<String>) {
        (Pipeline::Pending(LoadListeners::new()), Some(key.to_owned()))
    }
}

pub type PipelineLoadRequest = String;

pub enum PipelineLoadResponse {
    Error(String),
    Descriptor(PipelineDescriptor),
    ShaderReady(ShaderType),
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
    ) -> Option<PipelineLoadResponse> {
        if cancellation_token.is_canceled() {
            return None;
        }

        let url = match self.base_url.join(&source_id) {
            Err(err) => {
                let err = format!("Invalid pipeline url ({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResponse::Error(err));
            }
            Ok(url) => url,
        };

        let data = match utils::assets::download_string(&url).await {
            Err(err) => {
                let err = format!("Failed to get pipeline({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResponse::Error(err));
            }
            Ok(data) => data,
        };
        log::trace!("pipeline [{}]: {}", source_id, data);

        let descriptor = match serde_json::from_str(&data) {
            Err(err) => {
                let err = format!("Failed to parse pipeline({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResponse::Error(err));
            }
            Ok(descriptor) => descriptor,
        };

        Some(PipelineLoadResponse::Descriptor(descriptor))
    }
}

impl DataLoader<Pipeline> for PipelineLoader {
    fn load<'a>(
        &'a mut self,
        pipeline_id: String,
        cancellation_token: CancellationToken<Pipeline>,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<PipelineLoadResponse>> + Send + 'a>> {
        self.load_from_url(cancellation_token, pipeline_id).boxed()
    }
}

impl<'a> DataUpdater<'a, Pipeline> for (&Context, &ShaderStore) {
    fn update<'u>(
        &mut self,
        load_context: LoadContext<'u, Pipeline>,
        data: &mut Pipeline,
        load_response: PipelineLoadResponse,
    ) -> Option<PipelineLoadRequest> {
        data.on_load(load_context, self.0, &mut self.1.read(), load_response)
    }
}

pub type PipelineStore = Store<Pipeline>;
pub type PipelineStoreRead<'a> = ReadGuard<'a, Pipeline>;
pub type PipelineIndex = Index<Pipeline>;
