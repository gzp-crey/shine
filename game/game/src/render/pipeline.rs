use crate::assets::{
    AssetError, AssetIO, IntoVertexTypeId, PipelineBuffer, PipelineDescriptor, ShaderType, Url, UrlError,
    VertexBufferLayout, VertexBufferLayouts, VertexTypeId,
};
use crate::render::{Context, ShaderDependency, ShaderStore, ShaderStoreRead};
use shine_ecs::core::store::{
    AsyncLoadHandler, AsyncLoader, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading, ReadGuard, Store,
};
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;

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

/// Render pipeline
pub enum Pipeline {
    Requested(Url, VertexTypeId/*, LoadListeners*/),
    WaitingDependency(WaitingDependency/*, LoadListeners*/),
    Compiled(PipelineBuffer),
    Error,
}

impl Pipeline {
    pub fn pipeline_buffer(&self) -> Option<&PipelineBuffer> {
        if let Pipeline::Compiled(ref pipeline_buffer) = self {
            Some(pipeline_buffer)
        } else {
            None
        }
    }
}

impl Data for Pipeline {
    type Key = PipelineKey;
}

impl FromKey for Pipeline {
    fn from_key(key: &PipelineKey) -> Self {
        let vertex_layouts = pipeline_key.vertex_type.to_layout();
        match Url::parse(&key) {
            Ok(url) => Pipeline::Requested(url, vertex_layouts /*, LoadListener::new()*/),
            Err(err) => {
                log::warn!("Invalid pipeline url ({}): {:?}", key, err);
                Pipeline::Error
            }
        }
    }
}
/*
impl<'a> Update<'a> for Pipeline {
    type UpdateContext = (&'a Context, &'a ShaderStore);
    type LoadRequest = PipelineLoadRequest;
    type LoadResponse = PipelineLoadResponse;

    fn update(
        &mut self,
        load_context: LoadContext<'_, Pipeline>,
        update_context: Self::UpdateContext,
        load_response: PipelineLoadResponse,
    ) -> Option<PipelineLoadRequest> {
        let (context, shaders) = (update_context.0, &mut update_context.1.read());
        *self = match (std::mem::replace(self, Pipeline::None), load_response) {
            (Pipeline::Pending(listeners), Err(err)) => {
                log::debug!("Pipeline compilation failed [{:?}]: {:?}", load_context, err);
                listeners.notify_all();
                Pipeline::Error
            }

            (Pipeline::Pending(listeners), Ok(PipelineLoadData::Descriptor(descriptor, vertex_layouts))) => {
                let pipeline = PartialPipeline::from_descriptor(&load_context, descriptor, vertex_layouts, shaders);
                pipeline.into_pipeline(&load_context, context, shaders, listeners)
            }

            (Pipeline::WaitingDependency(pipeline, listeners), Ok(PipelineLoadData::ShaderReady(shader_type))) => {
                pipeline
                    .with_updated_shader_dependency(shader_type, shaders)
                    .into_pipeline(&load_context, context, shaders, listeners)
            }

            (Pipeline::Error, Ok(PipelineLoadData::ShaderReady(_))) => Pipeline::Error,

            _ => unreachable!(),
        };

        None
    }
}
*/

/// Partially loaded pipeline and pipeline dependency tracking
pub struct WaitingDependency {
    descriptor: Box<PipelineDescriptor>,
    vertex_layouts: VertexBufferLayouts,
    vertex_shader: ShaderDependency,
    fragment_shader: ShaderDependency,
}

impl WaitingDependency {
    fn from_descriptor(
        descriptor: Box<PipelineDescriptor>,
        vertex_layouts: VertexBufferLayouts,
    ) -> PartialPipeline {
        PartialPipeline {
            descriptor,
            vertex_layouts,
            vertex_shader : ShaderDependency::from_key(ShaderType::Vertex,descriptor.vertex_stage.shader.clone()),
            fragment_shader: ShaderDependency::from_key(ShaderType::Fragment,descriptor.fragment_stage.shader.clone()),
        }
    }

    fn into_pipeline(
        self,
        &load_handler: AsyncLoadHandler<Pipeline>,
        load_token: &LoadToken<Pipeline>,
        context: &Context,
        shaders: &mut ShaderStoreRead<'_>,
        //listeners: LoadListeners,
    ) -> Pipeline {
        let vs = match self.vertex_shader.request(shaders, ||{} ) {
            Ok(vs) => vs,
            Err(err) => {
                log::warn!("[{:?}] Pipeline vertex shader dependency failed: {:?}", load_token, err);
                return Pipeline::Error
            }
        };

        let fs = match self.fragment_shader.request(shaders, ||{} ) {
            Ok(fs) => fs,
            Err(err) => {
                log::warn!("[{:?}] Pipeline fragment shader dependency failed: {:?}", load_token, err);
                return Pipeline::Error
            }
        };

        if let (Some(vs), Some(fs)) = (vs,fs) {
            //listeners.notify_all();
            self.descriptor.to_pipeline_buffer(
                context.device(),
                context.swap_chain_format(),
                &self.vertex_layouts,
                |stage| match stage {
                    ShaderType::Vertex => Ok(shaders.at(vs).shadere_module().unwrap()),
                    ShaderType::Fragment => Ok(shaders.at(fs).shadere_module().unwrap()),
                    _ => unreachable!(),
                },
            ) {
                Ok(pipeline) => {
                    log::debug!("[{:?}] Pipeline compilation completed", load_token);
                    Pipeline::Compiled(pipeline)
                }
                Err(err) => {
                    log::warn!("[{:?}] Pipeline compilation failed: {:?}", load_token, err);
                    Pipeline::Error
                }
            }
        }
        else {
            load_handler.request(load_token, WaitDependencies(deps));
            WaitingDependency( self )
        }
    }
}
/*
pub enum PipelineLoadData {
    Descriptor(Box<PipelineDescriptor>, VertexBufferLayouts),
    ShaderReady(ShaderType),
}
*/
pub enum PipelineLoadRequest {
    LoadResource(Url, VertexLayouts)
    WaitDependency
};

pub type PipelineLoadResponse = Result<PipelineLoadData, PipelineLoadError>;

/// Error during pipeline load
#[derive(Debug)]
pub enum PipelineLoadError {
    Asset(AssetError),
    DependencyFailed,
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
/*
pub struct PipelineLoader {
    assetio: Arc<AssetIO>,
}

impl PipelineLoader {
    pub fn new(assetio: Arc<AssetIO>) -> PipelineLoader {
        PipelineLoader { assetio }
    }

    async fn load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Pipeline>,
        pipeline_key: PipelineKey,
    ) -> PipelineLoadResponse {
        if cancellation_token.is_canceled() {
            return Err(PipelineLoadError::Canceled);
        }

        let source_id = &pipeline_key.name;
        let url = Url::parse(&source_id)?;
        log::debug!("[{}] Loading pipeline...", url.as_str());

        let vertex_layouts = pipeline_key.vertex_type.to_layout();
        log::trace!("Vertex attributes: {:#?}", vertex_layouts);

        let data = self.assetio.download_binary(&url).await?;
        let descriptor = bincode::deserialize::<PipelineDescriptor>(&data)?;
        log::trace!("pipeline: {:#?}", descriptor);

        descriptor.vertex_stage.check_vertex_layouts(&vertex_layouts)?;
        Ok(PipelineLoadData::Descriptor(Box::new(descriptor), vertex_layouts))
    }
}

impl DataLoader<Pipeline> for PipelineLoader {
    fn load<'a>(
        &'a mut self,
        pipeline_key: PipelineKey,
        cancellation_token: CancellationToken<Pipeline>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<PipelineLoadResponse>>>> {
        Box::pin(async move {
            match self.load_from_url(cancellation_token, pipeline_key).await {
                Err(PipelineLoadError::Canceled) => None,
                result => Some(result),
            }
        })
    }
}

pub type PipelineStore = Store<Pipeline>;
pub type PipelineStoreRead<'a> = ReadGuard<'a, Pipeline>;
pub type PipelineIndex = Index<Pipeline>;
pub type PipelineId = GeneralId<Pipeline>;
*/
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
                pipelines.load_and_finalize_requests(&*context);
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
