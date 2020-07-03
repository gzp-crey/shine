use crate::assets::{
    AssetError, AssetIO, FrameGraphBuffer, FrameGraphDescriptor, Url, UrlError,
};
use crate::render::{Context, PipelineStore, PipelineStoreRead};
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, DataUpdater, FromKey, GeneralId, Index, LoadContext, LoadListeners, ReadGuard,
    Store,
};
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;

/// Error during frame graph loading
#[derive(Debug)]
pub enum FrameGraphLoadError {
    Asset(AssetError),
    Canceled,
}

impl From<UrlError> for FrameGraphLoadError {
    fn from(err: UrlError) -> FrameGraphLoadError {
        FrameGraphLoadError::Asset(AssetError::InvalidUrl(err))
    }
}

impl From<AssetError> for FrameGraphLoadError {
    fn from(err: AssetError) -> FrameGraphLoadError {
        FrameGraphLoadError::Asset(err)
    }
}

impl From<bincode::Error> for FrameGraphLoadError {
    fn from(err: bincode::Error) -> FrameGraphLoadError {
        FrameGraphLoadError::Asset(AssetError::ContentLoad(format!("Binary stream error: {}", err)))
    }
}

/// Frame graph dependencies required for compilation
pub struct PartialFrameGraph {
    descriptor: Box<FrameGraphDescriptor>,
}

impl PartialFrameGraph {
    fn from_descriptor(
        load_context: &LoadContext<'_, FrameGraph>,
        descriptor: Box<FrameGraphDescriptor>,
        pipelines: &mut PipelineStoreRead<'_>,
    ) -> PartialFrameGraph {
        PartialFrameGraph {
            descriptor,
        }
    }

    fn into_frame_graph(
        self,
        load_context: &LoadContext<'_, FrameGraph>,
        context: &Context,
        pipelines: &mut PipelineStoreRead<'_>,
        listeners: LoadListeners,
    ) -> FrameGraph {
        unimplemented!()
    }
}

pub enum FrameGraph {
    Pending(LoadListeners),
    WaitingDependency(PartialFrameGraph, LoadListeners),
    Compiled(FrameGraphBuffer),
    Error,
    None,
}

impl FrameGraph {
    pub fn frame_graph_buffer(&self) -> Option<&FrameGraphBuffer> {
        if let FrameGraph::Compiled(ref frame_graph_buffer) = self {
            Some(frame_graph_buffer)
        } else {
            None
        }
    }

    fn on_update(
        &mut self,
        load_context: LoadContext<'_, FrameGraph>,
        context: &Context,
        pipelines: &mut PipelineStoreRead<'_>,
        load_response: FrameGraphLoadResponse,
    ) -> Option<FrameGraphKey> {
        *self = match (std::mem::replace(self, FrameGraph::None), load_response) {
            (FrameGraph::Pending(listeners), Err(err)) => {
                log::debug!("Frame graph compilation failed [{:?}]: {:?}", load_context, err);
                listeners.notify_all();
                FrameGraph::Error
            }

            (FrameGraph::Pending(listeners), Ok(FrameGraphLoadData::Descriptor(descriptor))) => {
                let frame_graph = PartialFrameGraph::from_descriptor(&load_context, descriptor, pipelines);
                frame_graph.into_frame_graph(&load_context, context, pipelines, listeners)
            }

            (FrameGraph::WaitingDependency(frame_graph, listeners), Ok(FrameGraphLoadData::PipelineReady(pipeline_id))) => {
                unimplemented!()
                /*pipeline
                    .with_updated_shader_dependency(pipeline_id, pipelines)
                    .into_frame_graph(&load_context, context, pipelines, listeners)*/
            }

            (FrameGraph::Error, Ok(FrameGraphLoadData::PipelineReady(_))) => FrameGraph::Error,

            _ => unreachable!(),
        };

        None
    }
}

pub type FrameGraphKey  = String;

impl Data for FrameGraph {
    type Key = FrameGraphKey;
    type LoadRequest = FrameGraphLoadRequest;
    type LoadResponse = FrameGraphLoadResponse;
}

impl FromKey for FrameGraph {
    fn from_key(key: &FrameGraphKey) -> (Self, Option<FrameGraphKey>) {
        (FrameGraph::Pending(LoadListeners::new()), Some(key.to_owned()))
    }
}

pub enum FrameGraphLoadData {
    Descriptor(Box<FrameGraphDescriptor>),
    PipelineReady(String),
}

pub type FrameGraphLoadRequest = FrameGraphKey;
pub type FrameGraphLoadResponse = Result<FrameGraphLoadData, FrameGraphLoadError>;

pub struct FrameGraphLoader {
    assetio: Arc<AssetIO>,
}

impl FrameGraphLoader {
    pub fn new(assetio: Arc<AssetIO>) -> FrameGraphLoader {
        FrameGraphLoader { assetio }
    }

    async fn load_from_url(
        &mut self,
        cancellation_token: CancellationToken<FrameGraph>,
        source_id: FrameGraphKey,
    ) -> FrameGraphLoadResponse {
        if cancellation_token.is_canceled() {
            return Err(FrameGraphLoadError::Canceled);
        }

        let url = Url::parse(&source_id)?;
        log::debug!("[{}] Loading frame graph ...", url.as_str());

        let data = self.assetio.download_binary(&url).await?;
        let descriptor = bincode::deserialize::<FrameGraphDescriptor>(&data)?;
        log::trace!("frame_graph: {:#?}", descriptor);

        Ok(FrameGraphLoadData::Descriptor(Box::new(descriptor)))
    }

    async fn try_load_from_url(
        &mut self,
        cancellation_token: CancellationToken<FrameGraph>,
        frame_graph_key: FrameGraphKey,
    ) -> Option<FrameGraphLoadResponse> {
        match self.load_from_url(cancellation_token, frame_graph_key).await {
            Err(FrameGraphLoadError::Canceled) => None,
            result => Some(result),
        }
    }
}

impl DataLoader<FrameGraph> for FrameGraphLoader {
    fn load<'a>(
        &'a mut self,
        frame_graph_key: FrameGraphKey,
        cancellation_token: CancellationToken<FrameGraph>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<FrameGraphLoadResponse>>>> {
        Box::pin(self.try_load_from_url(cancellation_token, frame_graph_key))
    }
}

impl<'a> DataUpdater<'a, FrameGraph> for (&Context, &PipelineStore) {
    fn update<'u>(
        &mut self,
        load_context: LoadContext<'u, FrameGraph>,
        data: &mut FrameGraph,
        load_response: FrameGraphLoadResponse,
    ) -> Option<FrameGraphLoadRequest> {
        data.on_update(load_context, self.0, &mut self.1.read(), load_response)
    }
}

pub type FrameGraphStore = Store<FrameGraph>;
pub type FrameGraphStoreRead<'a> = ReadGuard<'a, FrameGraph>;
pub type FrameGraphIndex = Index<FrameGraph>;
pub type FrameGraphId = GeneralId<FrameGraph>;

pub mod systems {
    use super::*;
    use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

    pub fn update_frame_graphs() -> Box<dyn Schedulable> {
        SystemBuilder::new("update_frame_graphs")
            .read_resource::<Context>()
            .read_resource::<PipelineStore>()
            .write_resource::<FrameGraphStore>()
            .build(move |_, _, (context, pipelines, frame_graphs), _| {
                //log::info!("frame_graph");
                let mut frame_graphs = frame_graphs.write();
                let context: &Context = &*context;
                let pipelines: &PipelineStore = &*pipelines;
                frame_graphs.update(&mut (context, pipelines));
                frame_graphs.finalize_requests();
            })
    }

    pub fn gc_frame_graphs() -> Box<dyn Schedulable> {
        SystemBuilder::new("gc_frame_graphs")
            .write_resource::<FrameGraphStore>()
            .build(move |_, _, frame_graphs, _| {
                let mut frame_graphs = frame_graphs.write();
                frame_graphs.drain_unused();
            })
    }
}
