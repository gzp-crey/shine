use crate::{
    assets::{
        AssetIO, AssetId, CookedPipeline, PipelineStateDescriptor, Url, VertexBufferDescriptor, VertexBufferLayout,
    },
    render::{Compile, CompiledPipeline},
};
use serde::{Deserialize, Serialize};
use shine_ecs::{
    core::observer::ObserveDispatcher,
    resources::{
        ResourceHandle, ResourceId, ResourceKeyHandle, ResourceLoadRequester, ResourceLoadResponder, ResourceLoader,
        Resources,
    },
    ECSError,
};
use std::sync::Arc;

pub struct PipelineError;

/// Unique key for a pipeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineKey {
    pub id: String,
    pub vertex_layouts: Vec<VertexBufferLayout>,
    pub render_state: PipelineStateDescriptor,
}

impl PipelineKey {
    pub fn new<V: VertexBufferDescriptor>(id: String, render_state: PipelineStateDescriptor) -> PipelineKey {
        PipelineKey {
            id: id,
            vertex_layouts: <V as VertexBufferDescriptor>::buffer_layouts(),
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
    //vertex_shader: ShaderDependency,
    //fragment_shader: ShaderDependency,
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

    /*pub fn pipeline_module(&self) -> Option<&CompiledPipeline> {
        self.pipeline.as_ref().map(|u| u.as_ref()).unwrap_or(None)
    }*/
}

struct LoadRequest(String);

enum LoadResponse {
    Compiled(CompiledPipeline),
    Error(PipelineError),
    RequestShader(AssetId),
}

/// Implement functions to make it a resource
impl Pipeline {
    fn build(
        context: &ResourceLoadRequester<Self, LoadRequest>,
        handle: ResourceHandle<Self>,
        id: &ResourceId,
    ) -> Self {
        log::trace!("Creating [{:?}]", id);
        if let Ok(PipelineKey {
            id,
            vertex_layouts,
            render_state,
        }) = id.to_object::<PipelineKey>()
        {
            context.send_request(handle, LoadRequest(id.clone()));
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
        responder: &ResourceLoadResponder<Pipeline, LoadResponse>,
        (io, device): &(AssetIO, Arc<wgpu::Device>),
        handle: &ResourceHandle<Self>,
        pipeline_id: String,
    ) -> Result<CompiledPipeline, PipelineError> {
        log::debug!("[{:?}] Loading pipeline...", pipeline_id);

        let url = Url::parse(&pipeline_id).map_err(|_| PipelineError)?;
        let data = io.download_binary(&url).await.map_err(|_| PipelineError)?;

        log::debug!("[{:?}] Extracting pipeline...", pipeline_id);
        handle.check_liveness().map_err(|_| PipelineError)?;
        let cooked_pipeline: CookedPipeline = bincode::deserialize_from(&*data).map_err(|_| PipelineError)?;
        handle.check_liveness().map_err(|_| PipelineError)?;

        let fs = AssetId::new(cooked_pipeline.descriptor.fragment_stage.shader).map_err(|_| PipelineError)?;
        let vs = AssetId::new(cooked_pipeline.descriptor.vertex_stage.shader).map_err(|_| PipelineError)?;
        responder.send_response(handle.clone(), LoadResponse::RequestShader(fs));
        responder.send_response(handle.clone(), LoadResponse::RequestShader(vs));
        handle.check_liveness().map_err(|_| PipelineError)?;

        // send request and wait shaders
        // compile pipeline

        /*log::debug!("[{:?}] Compiling pipeline...", pipeline_id);
        handle.check_liveness().map_err(|_| PipelineError)?;
        let compiled_pipeline = cooked_pipeline.compile(&*device);

        log::debug!("[{:?}] Pipeline loaded", pipeline_id);
        Ok(compiled_pipeline)*/
        Err(PipelineError)
    }

    async fn on_load(
        ctx: &(AssetIO, Arc<wgpu::Device>),
        responder: &ResourceLoadResponder<Pipeline, LoadResponse>,
        handle: ResourceHandle<Pipeline>,
        request: LoadRequest,
    ) {
        let LoadRequest(pipeline_id) = request;
        let response = match Self::on_load_impl(responder, ctx, &handle, pipeline_id).await {
            Ok(pipeline) => LoadResponse::Compiled(pipeline),
            Err(err) => LoadResponse::Error(err),
        };
        responder.send_response(handle, response);
    }

    fn on_load_response(
        this: &mut Self,
        _requester: &ResourceLoadRequester<Self, LoadRequest>,
        _handle: &ResourceHandle<Self>,
        response: LoadResponse,
    ) {
        log::debug!("[{:?}] Load response", this.id);
        match response {
            LoadResponse::Compiled(shader) => this.pipeline = Ok(Some(shader)),
            LoadResponse::Error(err) => this.pipeline = Err(err),
            LoadResponse::RequestShader(sh) => unimplemented!(),
        };
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

pub type PipelineHandle = ResourceHandle<Pipeline>;
pub type PipelineDependency = ResourceKeyHandle<PipelineKey, Pipeline>;
