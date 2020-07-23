use crate::assets::{
    AssetError, AssetIO, IntoVertexTypeId, PipelineDescriptor, ShaderType, Url, UrlError, VertexBufferLayouts,
    VertexTypeId,
};
use crate::render::{Compile, Context, PipelineBuffer, ShaderDependency, ShaderStore, ShaderStoreRead};
use shine_ecs::core::store::{
    AsyncLoadHandler, AsyncLoader, AutoNamedId, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading,
    ReadGuard, Store,
};
use std::fmt;
use std::pin::Pin;

/// Unique key for a render pipeline.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PipelineKey {
    pub name: String,
    pub vertex_type: VertexTypeId,
    //pub render_target: RenderTargetId,
}

impl PipelineKey {
    pub fn new<V: IntoVertexTypeId>(name: &str) -> PipelineKey {
        PipelineKey {
            name: name.to_owned(),
            vertex_type: <V as IntoVertexTypeId>::into_id(),
        }
    }
}

impl fmt::Debug for PipelineKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("").field(&self.name).field(&self.vertex_type).finish()
    }
}

pub enum CompiledPipeline {
    None,
    Error,
    Compiled(PipelineBuffer),
}

impl CompiledPipeline {}

pub struct Pipeline {
    id: String,
    vertex_layouts: VertexBufferLayouts,
    descriptor: Option<PipelineDescriptor>,
    vertex_shader: ShaderDependency,
    fragment_shader: ShaderDependency,
    pipeline: CompiledPipeline,
}

impl Pipeline {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn pipeline(&self) -> &CompiledPipeline {
        &self.pipeline
    }

    pub fn pipeline_buffer(&self) -> Option<&PipelineBuffer> {
        if let CompiledPipeline::Compiled(pipeline) = &self.pipeline {
            Some(pipeline)
        } else {
            None
        }
    }

    fn recompile(
        &mut self,
        load_handler: &mut AsyncLoadHandler<Pipeline>,
        context: &Context,
        shaders: &mut ShaderStoreRead<'_>,
        load_token: LoadToken<Pipeline>,
    ) {
        // update dependencies
        self.vertex_shader.request(shaders, |listeners| {
            listeners.add(
                load_handler,
                load_token.clone(),
                PipelineLoadResponse::shader_loaded(ShaderType::Vertex),
            )
        });
        self.fragment_shader.request(shaders, |listeners| {
            listeners.add(
                load_handler,
                load_token.clone(),
                PipelineLoadResponse::shader_loaded(ShaderType::Fragment),
            )
        });

        // try to compile
        if let Some(descriptor) = &self.descriptor {
            let vs = match self.vertex_shader.shader_module(shaders) {
                Err(_) => {
                    self.pipeline = CompiledPipeline::Error;
                    log::warn!("[{:?}] Pipeline vertex shader dependency failed", load_token);
                    return;
                }
                Ok(None) => {
                    log::debug!("[{:?}] Pipeline, missing vertex shader dependency", load_token);
                    self.pipeline = CompiledPipeline::None;
                    return;
                }
                Ok(Some(sh)) => sh,
            };

            let fs = match self.fragment_shader.shader_module(shaders) {
                Err(_) => {
                    self.pipeline = CompiledPipeline::Error;
                    log::warn!("[{:?}] Pipeline fragment shader dependency failed", load_token);
                    return;
                }
                Ok(None) => {
                    log::debug!("[{:?}] Pipeline, missing fragment shader dependency", load_token);
                    self.pipeline = CompiledPipeline::None;
                    return;
                }
                Ok(Some(sh)) => sh,
            };

            match descriptor.compile(
                context.device(),
                (context.swap_chain_format(), &self.vertex_layouts, |stage| match stage {
                    ShaderType::Vertex => Ok(vs),
                    ShaderType::Fragment => Ok(fs),
                    _ => unreachable!(),
                }),
            ) {
                Ok(pipeline) => {
                    self.pipeline = CompiledPipeline::Compiled(pipeline);
                    log::debug!("[{:?}] Pipeline compilation completed", load_token);
                }
                Err(err) => {
                    self.pipeline = CompiledPipeline::Error;
                    log::warn!("[{:?}] Pipeline compilation failed: {:?}", load_token, err);
                }
            };
        } else {
            log::debug!("[{:?}] Pipeline descriptor missing", load_token);
            self.pipeline = CompiledPipeline::None;
        }
    }
}

impl Data for Pipeline {
    type Key = PipelineKey;
}

impl FromKey for Pipeline {
    fn from_key(key: &PipelineKey) -> Self {
        Pipeline {
            id: key.name.clone(),
            vertex_layouts: key.vertex_type.to_layout(),
            descriptor: None,
            vertex_shader: ShaderDependency::unknown(),
            fragment_shader: ShaderDependency::unknown(),
            pipeline: CompiledPipeline::None,
        }
    }
}

impl<'l> OnLoading<'l> for Pipeline {
    type LoadingContext = (&'l Context, &'l ShaderStore);
}

impl OnLoad for Pipeline {
    type LoadRequest = PipelineLoadRequest;
    type LoadResponse = PipelineLoadResponse;
    type LoadHandler = AsyncLoadHandler<Self>;

    fn on_load_request(&mut self, load_handler: &mut Self::LoadHandler, load_token: LoadToken<Self>) {
        let request = PipelineLoadRequest(self.id.clone(), self.vertex_layouts.clone());
        load_handler.request(load_token, request);
    }

    fn on_load_response<'l>(
        &mut self,
        load_handler: &mut Self::LoadHandler,
        load_context: &mut (&'l Context, &'l ShaderStore),
        load_token: LoadToken<Self>,
        load_response: PipelineLoadResponse,
    ) {
        let (context, shaders) = (load_context.0, &mut load_context.1.read());
        match load_response.0 {
            Err(_) => {
                self.pipeline = CompiledPipeline::Error;
            }
            Ok(PipelineLoadResponseInner::PipelineDescriptor(desc)) => {
                if self
                    .descriptor
                    .as_ref()
                    .map(|old| old.vertex_stage.shader != desc.vertex_stage.shader)
                    .unwrap_or(true)
                {
                    log::trace!(
                        "[{:?}] Pipeline vertex shader altered: {:?}",
                        load_token,
                        desc.vertex_stage.shader
                    );
                    self.vertex_shader =
                        ShaderDependency::from_key(ShaderType::Vertex, desc.vertex_stage.shader.clone());
                }
                if self
                    .descriptor
                    .as_ref()
                    .map(|old| old.fragment_stage.shader != desc.fragment_stage.shader)
                    .unwrap_or(true)
                {
                    log::trace!(
                        "[{:?}] Pipeline fragment shader altered: {:?}",
                        load_token,
                        desc.fragment_stage.shader
                    );
                    self.fragment_shader =
                        ShaderDependency::from_key(ShaderType::Fragment, desc.fragment_stage.shader.clone());
                }
                self.descriptor = Some(*desc);
                self.recompile(load_handler, context, shaders, load_token);
            }
            Ok(PipelineLoadResponseInner::ShaderReady(ty)) => match ty {
                ShaderType::Vertex => {
                    log::debug!("[{:?}] Pipeline vertex shader loaded", load_token);
                    self.recompile(load_handler, context, shaders, load_token);
                }
                ShaderType::Fragment => {
                    log::debug!("[{:?}] Pipeline fragment shader loaded", load_token);
                    self.recompile(load_handler, context, shaders, load_token);
                }
                ty => {
                    log::warn!("[{:?}] Pipeline got invalid shader response: {:?}", load_token, ty);
                    self.pipeline = CompiledPipeline::Error;
                }
            },
        };
    }
}

pub struct PipelineLoadRequest(String, VertexBufferLayouts);

enum PipelineLoadResponseInner {
    PipelineDescriptor(Box<PipelineDescriptor>),
    ShaderReady(ShaderType),
}

pub struct PipelineLoadResponse(Result<PipelineLoadResponseInner, PipelineLoadError>);

impl PipelineLoadResponse {
    fn shader_loaded(ty: ShaderType) -> PipelineLoadResponse {
        PipelineLoadResponse(Ok(PipelineLoadResponseInner::ShaderReady(ty)))
    }
}

/// Error during pipeline load
#[derive(Debug)]
pub enum PipelineLoadError {
    Asset(AssetError),
    Canceled,
}

impl From<UrlError> for PipelineLoadError {
    fn from(err: UrlError) -> PipelineLoadError {
        PipelineLoadError::Asset(AssetError::InvalidUrl(err))
    }
}

impl From<LoadCanceled> for PipelineLoadError {
    fn from(_err: LoadCanceled) -> PipelineLoadError {
        PipelineLoadError::Canceled
    }
}

impl From<AssetError> for PipelineLoadError {
    fn from(err: AssetError) -> PipelineLoadError {
        PipelineLoadError::Asset(err)
    }
}

impl From<bincode::Error> for PipelineLoadError {
    fn from(err: bincode::Error) -> PipelineLoadError {
        PipelineLoadError::Asset(AssetError::ContentLoad(format!("Binary stream error: {}", err)))
    }
}

impl AssetIO {
    async fn load_pipeline(
        &self,
        load_token: LoadToken<Pipeline>,
        source_id: String,
        vertex_layouts: VertexBufferLayouts,
    ) -> Result<PipelineLoadResponseInner, PipelineLoadError> {
        let url = Url::parse(&source_id)?;
        log::debug!("[{:?}] Loading pipeline...", load_token);
        log::trace!("Vertex attributes: {:#?}", vertex_layouts);

        let data = self.download_binary(&url).await?;
        let descriptor = bincode::deserialize::<PipelineDescriptor>(&data)?;
        log::trace!("pipeline: {:#?}", descriptor);

        descriptor.vertex_stage.check_vertex_layouts(&vertex_layouts)?;
        Ok(PipelineLoadResponseInner::PipelineDescriptor(Box::new(descriptor)))
    }
}

impl AsyncLoader<Pipeline> for AssetIO {
    fn load<'l>(
        &'l mut self,
        load_token: LoadToken<Pipeline>,
        request: PipelineLoadRequest,
    ) -> Pin<Box<dyn 'l + std::future::Future<Output = Option<PipelineLoadResponse>>>> {
        Box::pin(async move {
            match self.load_pipeline(load_token, request.0, request.1).await {
                Err(PipelineLoadError::Canceled) => None,
                result => Some(PipelineLoadResponse(result)),
            }
        })
    }
}

pub type PipelineStore = Store<Pipeline, AsyncLoadHandler<Pipeline>>;
pub type PipelineStoreRead<'a> = ReadGuard<'a, Pipeline, AsyncLoadHandler<Pipeline>>;
pub type PipelineIndex = Index<Pipeline>;
pub type PipelineNamedId = AutoNamedId<Pipeline>;

pub mod systems {
    use super::*;
    use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

    pub fn update_pipelines() -> Box<dyn Schedulable> {
        SystemBuilder::new("update_pipelines")
            .read_resource::<Context>()
            .read_resource::<ShaderStore>()
            .write_resource::<PipelineStore>()
            .build(move |_, _, (context, shaders, pipelines), _| {
                //log::info!("pipeline");
                pipelines.load_and_finalize_requests((&*context, &*shaders));
            })
    }

    pub fn gc_pipelines() -> Box<dyn Schedulable> {
        SystemBuilder::new("gc_pipelines")
            .write_resource::<PipelineStore>()
            .build(move |_, _, pipelines, _| {
                pipelines.drain_unused();
            })
    }
}
