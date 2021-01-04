use crate::{
    assets::{AssetIO, CookedPipeline, Url},
    render::{Compile, CompiledPipeline},
};
use serde::{Deserialize, Serialize};
use shine_ecs::{
    core::observer::ObserveDispatcher,
    resources::{
        PipelineStateDescriptor, ResourceHandle, ResourceId, ResourceLoadRequester, ResourceLoader, Resources,
        ShaderDependency, VertexBufferDescriptor, VertexBufferLayouts,
    },
    ECSError,
};
use std::sync::Arc;

pub struct PipelineError;

/// Unique key for a pipeline
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PipelineKey {
    pub id: String,
    pub vertex_layouts: VertexBufferLayouts,
    pub render_state: PipelineStateDescriptor,
}

impl PipelineKey {
    pub fn new<V: VertexBufferDescriptor>(id: &str, render_state: &PipelineStateDescriptor) -> PipelineKey {
        PipelineKey {
            id: id.to_owned(),
            vertex_type: <V as IntoVertexBufferLayouts>::buffer_layouts(),
            render_state,
        }
    }
}

#[derive(Debug)]
pub enum PipelineEvent {
    Loaded,
}

pub struct Pipeline {
    id: String,
    pipeline: Result<Option<CompiledPipeline>, PipelineError>,
    vertex_shader: ShaderDependency,
    fragment_shader: ShaderDependency,
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

    /*pub fn Pipeline_module(&self) -> Option<&CompiledPipeline> {
        self.pipeline.as_ref().map(|u| u.as_ref()).unwrap_or(None)
    }*/
}

struct PipelineRequest(String);
struct PipelineResponse(Result<CompiledPipeline, PipelineError>);

/// Implement functions to make it a resource
impl Pipeline {
    fn build(
        context: &ResourceLoadRequester<Self, PipelineRequest>,
        handle: ResourceHandle<Self>,
        id: &ResourceId,
    ) -> Self {
        log::trace!("Creating [{:?}]", id);
        if let Ok(PipelineKey {
            is,
            vertex_layouts,
            render_state,
        }) = id.to_object::<PipelineKey>()
        {
            context.send_request(handle, PipelineRequest(id.clone()));
            Pipeline {
                id,
                pipeline: Ok(None),
                dispatcher: Default::default(),
            }
        } else {
            Pipeline {
                id: Default::default(),
                pipeline: Err(PipelineError),
                dispatcher: Default::default(),
            }
        }
    }

    async fn on_load_impl(
        (io, device): &(AssetIO, Arc<wgpu::Device>),
        handle: ResourceHandle<Self>,
        pipeline_id: String,
    ) -> Result<CompiledPipeline, PipelineError> {
        log::debug!("[{:?}] Loading pipeline...", pipeline_id);

        let url = Url::parse(&pipeline_id).map_err(|_| PipelineError)?;
        let data = io.download_binary(&url).await.map_err(|_| PipelineError)?;

        log::debug!("[{:?}] Extracting pipeline...", pipeline_id);
        handle.check_liveness().map_err(|_| PipelineError)?;
        let cooked_pipeline: CookedPipeline = bincode::deserialize_from(&*data).map_err(|_| PipelineError)?;

        /*log::debug!("[{:?}] Compiling pipeline...", pipeline_id);
        handle.check_liveness().map_err(|_| PipelineError)?;
        let compiled_pipeline = cooked_pipeline.compile(&*device);

        log::debug!("[{:?}] Pipeline loaded", pipeline_id);
        Ok(compiled_pipeline)*/
        Err(PipelineError)
    }

    async fn on_load(
        ctx: &(AssetIO, Arc<wgpu::Device>),
        handle: ResourceHandle<Self>,
        request: PipelineRequest,
    ) -> Option<PipelineResponse> {
        let PipelineRequest(pipeline_id) = request;
        Some(PipelineResponse(Self::on_load_impl(ctx, handle, pipeline_id).await))
    }

    fn on_load_response(
        this: &mut Self,
        _requester: &ResourceLoadRequester<Self, PipelineRequest>,
        _handle: &ResourceHandle<Self>,
        response: PipelineResponse,
    ) {
        log::debug!("[{:?}] Load completed (success: {})", this.id, response.0.is_ok());
        this.pipeline = response.0.map(Some);
        this.dispatcher.notify_all(PipelineEvent::Loaded);
    }

    pub fn register_resource(
        resources: &mut Resources,
        io: AssetIO,
        device: Arc<wgpu::Device>,
    ) -> Result<(), ECSError> {
        resources.register(ResourceLoader::new(
            Pipeline::build,
            (io, device),
            Pipeline::on_load,
            Pipeline::on_load_response,
        ))
    }

    pub fn unregister_resource(resources: &mut Resources) {
        resources.unregister::<Pipeline>();
    }

    pub fn bake_resource(resources: &mut Resources, gc: bool) {
        resources.bake::<Pipeline>(gc);
    }
}
