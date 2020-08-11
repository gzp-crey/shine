use crate::{
    assets::{
        AssetError, AssetIO, IntoVertexTypeId, PipelineDescriptor, PipelineStateDescriptor, PipelineStateTypeId,
        ShaderType, Url, UrlError, VertexBufferLayouts, VertexTypeId,
    },
    render::{Compile, CompiledPipeline, Context, ShaderDependency, ShaderStore, ShaderStoreRead},
};
use shine_ecs::core::{
    observer::{ObserveDispatcher, SubscriptionRequest},
    store::{
        AsyncLoadHandler, AsyncLoader, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading, ReadGuard,
        Store,
    },
};
use std::{fmt, mem, pin::Pin};

/// Unique key for a render pipeline.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PipelineKey {
    pub name: String,
    pub vertex_type: VertexTypeId,
    pub render_state: PipelineStateTypeId,
}

impl PipelineKey {
    pub fn new<V: IntoVertexTypeId>(name: &str, state: &PipelineStateDescriptor) -> PipelineKey {
        PipelineKey {
            name: name.to_owned(),
            vertex_type: <V as IntoVertexTypeId>::into_id(),
            render_state: PipelineStateTypeId::from_descriptor(state),
        }
    }
}

impl fmt::Debug for PipelineKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("")
            .field(&self.name)
            .field(&self.vertex_type)
            .field(&self.render_state)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct PipelineError;

pub enum PipelineEvent {
    Loaded,
}

pub struct Pipeline {
    id: String,
    vertex_layouts: VertexBufferLayouts,
    render_state: PipelineStateDescriptor,
    descriptor: Option<PipelineDescriptor>,
    vertex_shader: ShaderDependency,
    fragment_shader: ShaderDependency,
    pipeline: Result<Option<CompiledPipeline>, PipelineError>,
    dispatcher: ObserveDispatcher<PipelineEvent>,
}

impl Pipeline {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn dispatcher(&self) -> &ObserveDispatcher<PipelineEvent> {
        &self.dispatcher
    }

    pub fn pipeline(&self) -> Result<Option<&CompiledPipeline>, PipelineError> {
        match &self.pipeline {
            Err(_) => Err(PipelineError),
            Ok(None) => Ok(None),
            Ok(Some(pipeline)) => Ok(Some(pipeline)),
        }
    }

    pub fn pipeline_buffer(&self) -> Option<&CompiledPipeline> {
        if let Ok(Some(pipeline)) = &self.pipeline {
            Some(pipeline)
        } else {
            None
        }
    }

    fn recompile(&mut self, context: &Context, shaders: &mut ShaderStoreRead<'_>, load_token: LoadToken<Pipeline>) {
        // update dependencies
        self.vertex_shader.request(shaders);
        self.fragment_shader.request(shaders);

        // try to compile
        if let Some(descriptor) = &self.descriptor {
            let vs = match self.vertex_shader.shader_module(shaders) {
                Err(_) => {
                    self.pipeline = Err(PipelineError);
                    log::warn!("[{:?}] Pipeline vertex shader dependency failed", load_token);
                    return;
                }
                Ok(None) => {
                    log::debug!(
                        "[{:?}] Pipeline, missing (non-optional) vertex shader dependency",
                        load_token
                    );
                    self.pipeline = Ok(None);
                    return;
                }
                Ok(Some(sh)) => sh,
            };

            let fs = match self.fragment_shader.shader_module(shaders) {
                Err(_) => {
                    self.pipeline = Err(PipelineError);
                    log::warn!("[{:?}] Pipeline fragment shader dependency failed", load_token);
                    return;
                }
                Ok(None) => {
                    log::debug!(
                        "[{:?}] Pipeline, missing (non-optional) fragment shader dependency",
                        load_token
                    );
                    self.pipeline = Ok(None);
                    return;
                }
                Ok(Some(sh)) => sh,
            };

            match descriptor.compile(
                context.device(),
                (context.swap_chain_format(), &self.vertex_layouts, |stage| match stage {
                    ShaderType::Vertex => Ok(&vs.shader),
                    ShaderType::Fragment => Ok(&fs.shader),
                    _ => unreachable!(),
                }),
            ) {
                Ok(pipeline) => {
                    self.pipeline = Ok(Some(pipeline));
                    log::debug!("[{:?}] Pipeline compilation completed", load_token);
                    self.dispatcher.notify_all(PipelineEvent::Loaded);
                }
                Err(err) => {
                    self.pipeline = Err(PipelineError);
                    log::warn!("[{:?}] Pipeline compilation failed: {:?}", load_token, err);
                    self.dispatcher.notify_all(PipelineEvent::Loaded);
                }
            };
        } else {
            log::debug!("[{:?}] Pipeline descriptor missing", load_token);
            self.pipeline = Ok(None);
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
            vertex_layouts: key.vertex_type.to_layouts(),
            render_state: key.render_state.to_descriptor(),
            descriptor: None,
            vertex_shader: ShaderDependency::none(),
            fragment_shader: ShaderDependency::none(),
            pipeline: Ok(None),
            dispatcher: Default::default(),
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
        let request = PipelineLoadRequest(self.id.clone());
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
            PipelineLoadResponseInner::Error(err) => {
                log::warn!("[{:?}] Pipeline compilation failed: {:?}", load_token, err);
                self.pipeline = Err(PipelineError);
                self.dispatcher.notify_all(PipelineEvent::Loaded);
            }
            PipelineLoadResponseInner::PipelineDescriptor(desc) => {
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
                    self.vertex_shader = ShaderDependency::new()
                        .with_type(ShaderType::Vertex)
                        .with_id(desc.vertex_stage.shader.clone())
                        .with_load_response(
                            load_handler,
                            &load_token,
                            PipelineLoadResponse(PipelineLoadResponseInner::ShaderReady(ShaderType::Vertex)),
                        );
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
                    self.fragment_shader = ShaderDependency::new()
                        .with_type(ShaderType::Fragment)
                        .with_id(desc.fragment_stage.shader.clone())
                        .with_load_response(
                            load_handler,
                            &load_token,
                            PipelineLoadResponse(PipelineLoadResponseInner::ShaderReady(ShaderType::Fragment)),
                        );
                }
                self.descriptor = Some(*desc);
                self.recompile(context, shaders, load_token);
            }
            PipelineLoadResponseInner::ShaderReady(ty) => match ty {
                ShaderType::Vertex => {
                    log::debug!("[{:?}] Pipeline vertex shader loaded", load_token);
                    self.recompile(context, shaders, load_token);
                }
                ShaderType::Fragment => {
                    log::debug!("[{:?}] Pipeline fragment shader loaded", load_token);
                    self.recompile(context, shaders, load_token);
                }
                ty => {
                    log::warn!("[{:?}] Pipeline got invalid shader response: {:?}", load_token, ty);
                    self.pipeline = Err(PipelineError);
                    self.dispatcher.notify_all(PipelineEvent::Loaded);
                }
            },
        };
    }
}

//impl Observer<ShaderEvent> for ()

pub struct PipelineLoadRequest(String);

#[derive(Clone)]
enum PipelineLoadResponseInner {
    PipelineDescriptor(Box<PipelineDescriptor>),
    ShaderReady(ShaderType),
    Error(PipelineLoadError),
}

#[derive(Clone)]
pub struct PipelineLoadResponse(PipelineLoadResponseInner);

/// Error during pipeline load
#[derive(Debug, Clone)]
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
    ) -> Result<PipelineLoadResponseInner, PipelineLoadError> {
        let url = Url::parse(&source_id)?;
        log::debug!("[{:?}] Loading pipeline...", load_token);

        let data = self.download_binary(&url).await?;
        let descriptor = bincode::deserialize::<PipelineDescriptor>(&data)?;
        log::trace!("pipeline: {:#?}", descriptor);

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
            match self.load_pipeline(load_token, request.0).await {
                Err(PipelineLoadError::Canceled) => None,
                Err(err) => Some(PipelineLoadResponse(PipelineLoadResponseInner::Error(err))),
                Ok(result) => Some(PipelineLoadResponse(result)),
            }
        })
    }
}

pub type PipelineStore = Store<Pipeline, AsyncLoadHandler<Pipeline>>;
pub type PipelineStoreRead<'a> = ReadGuard<'a, Pipeline, AsyncLoadHandler<Pipeline>>;
pub type PipelineIndex = Index<Pipeline>;

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

/// Error indicating a failed pipeline dependency request.
pub struct PipelineDependencyError;

enum PipelineDependencyIndex {
    None,
    Incomplete,
    Pending(PipelineIndex),
    Completed(PipelineIndex),
    Error(PipelineDependencyError),
}

/// Helper to manage dependency on a pipeline
pub struct PipelineDependency {
    vertex_id: Option<VertexTypeId>,
    state_id: Option<PipelineStateTypeId>,
    id: Option<String>,
    index: PipelineDependencyIndex,
    subscription: SubscriptionRequest<PipelineEvent>,
}

impl PipelineDependency {
    pub fn none() -> PipelineDependency {
        PipelineDependency {
            vertex_id: None,
            state_id: None,
            id: None,
            index: PipelineDependencyIndex::None,
            subscription: SubscriptionRequest::None,
        }
    }

    pub fn new() -> PipelineDependency {
        PipelineDependency {
            vertex_id: None,
            state_id: None,
            id: None,
            index: PipelineDependencyIndex::Incomplete,
            subscription: SubscriptionRequest::None,
        }
    }

    pub fn with_vertex_layout<V: IntoVertexTypeId>(self) -> PipelineDependency {
        assert!(self.is_none());
        PipelineDependency {
            vertex_id: Some(<V as IntoVertexTypeId>::into_id()),
            index: PipelineDependencyIndex::Incomplete,
            subscription: self.subscription.with_cancel_active(),
            ..self
        }
    }

    pub fn with_state(self, state: &PipelineStateDescriptor) -> PipelineDependency {
        assert!(self.is_none());
        PipelineDependency {
            state_id: Some(PipelineStateTypeId::from_descriptor(state)),
            index: PipelineDependencyIndex::Incomplete,
            subscription: self.subscription.with_cancel_active(),
            ..self
        }
    }

    pub fn with_id<S: ToString>(self, id: S) -> PipelineDependency {
        assert!(self.is_none());
        PipelineDependency {
            id: Some(id.to_string()),
            index: PipelineDependencyIndex::Incomplete,
            subscription: self.subscription.with_cancel_active(),
            ..self
        }
    }

    pub fn key(&self) -> Result<PipelineKey, PipelineDependencyError> {
        if self.id.is_none() {
            log::warn!("Missing id");
            Err(PipelineDependencyError)
        } else if self.vertex_id.is_none() {
            log::warn!("Missing vertex layout");
            Err(PipelineDependencyError)
        } else if self.state_id.is_none() {
            log::warn!("Missing state");
            Err(PipelineDependencyError)
        } else {
            Ok(PipelineKey {
                name: self.id.clone().unwrap(),
                vertex_type: self.vertex_id.clone().unwrap(),
                render_state: self.state_id.clone().unwrap(),
            })
        }
    }

    pub fn is_none(&self) -> bool {
        if let PipelineDependencyIndex::None = self.index {
            true
        } else {
            false
        }
    }

    pub fn request(&mut self, pipelines: &mut PipelineStoreRead<'_>) {
        self.index = match mem::replace(&mut self.index, PipelineDependencyIndex::Incomplete) {
            PipelineDependencyIndex::Incomplete => match self.key() {
                Err(err) => PipelineDependencyIndex::Error(err),
                Ok(id) => PipelineDependencyIndex::Pending(pipelines.get_or_load(&id)),
            },
            PipelineDependencyIndex::Pending(idx) => {
                let pipeline = pipelines.at(&idx);
                self.subscription.subscribe(&pipeline.dispatcher);
                match pipeline.pipeline() {
                    Err(_) => PipelineDependencyIndex::Error(PipelineDependencyError),
                    Ok(None) => PipelineDependencyIndex::Pending(idx),
                    Ok(Some(_)) => PipelineDependencyIndex::Completed(idx),
                }
            }
            PipelineDependencyIndex::None => PipelineDependencyIndex::None,
            PipelineDependencyIndex::Completed(idx) => PipelineDependencyIndex::Completed(idx),
            PipelineDependencyIndex::Error(err) => PipelineDependencyIndex::Error(err),
        }
    }
}

impl Default for PipelineDependency {
    fn default() -> Self {
        Self::new()
    }
}
