use crate::assets::{AssetError, AssetIO, FrameGraphDescriptor, Url, UrlError};
use crate::render::{Context, Frame, FrameOutput, PipelineStore, PipelineStoreRead};
use shine_ecs::core::async_task::AsyncTask;
use std::fmt;

struct RenderTarget {
    name: String,
}

struct Pass {
    name: String,
    /*pub struct FramePassDescriptor {
        pub input: HashMap<String, SamplerDescriptor>,
        pub output: Vec<String>,
        pub method: FramePassMethod,
    }*/
}

pub struct FrameGraphBuffer {
    targets: Vec<RenderTarget>,
    passes: Vec<Pass>,
}

impl FrameGraphBuffer {
    pub fn from_descriptor(_descriptor: FrameGraphDescriptor) -> FrameGraphBuffer {
        FrameGraphBuffer {
            targets: Vec::new(),
            passes: Vec::new(),
        }
    }
}

enum CompiledFrameGraph {
    Error,
    Waiting(AsyncTask<Result<FrameGraphDescriptor, FrameGraphLoadError>>),
    Compiled(FrameGraphBuffer),
}

pub struct FrameGraph {
    graph: CompiledFrameGraph,
}

impl FrameGraph {
    pub fn from_descriptor(descriptor: FrameGraphDescriptor) -> FrameGraph {
        let graph = FrameGraphBuffer::from_descriptor(descriptor);
        FrameGraph {
            graph: CompiledFrameGraph::Compiled(graph),
        }
    }

    pub fn graph(&self) -> Option<&FrameGraphBuffer> {
        if let CompiledFrameGraph::Compiled(graph) = &self.graph {
            Some(graph)
        } else {
            None
        }
    }

    pub fn graph_mut(&mut self) -> Option<&mut FrameGraphBuffer> {
        if let CompiledFrameGraph::Compiled(graph) = &mut self.graph {
            Some(graph)
        } else {
            None
        }
    }

    pub fn update(&mut self) {
        let load_response = if let CompiledFrameGraph::Waiting(recv) = &mut self.graph {
            match recv.try_get() {
                Ok(None) => return,
                Err(_) => Err(FrameGraphLoadError::Canceled),
                Ok(Some(response)) => response,
            }
        } else {
            return;
        };

        match load_response {
            Err(_) => self.graph = CompiledFrameGraph::Error,
            Ok(descriptor) => self.graph = CompiledFrameGraph::Compiled(FrameGraphBuffer::from_descriptor(descriptor)),
        };
    }

    pub fn start_frame(&mut self) {}

    pub fn end_frame(&mut self) {}
}

/// Error during frame graph load
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

impl AssetIO {
    async fn load_frame_graph(&self, source_id: String) -> Result<FrameGraphDescriptor, FrameGraphLoadError> {
        let url = Url::parse(&source_id)?;
        log::debug!("[{:?}] Loading frame graph...", source_id);
        let data = self.download_binary(&url).await?;

        let descriptor = bincode::deserialize::<FrameGraphDescriptor>(&data)?;
        log::trace!("Graph: {:#?}", descriptor);
        Ok(descriptor)
    }
}

impl FrameGraph {
    pub fn load_from_url(assetio: AssetIO, descriptor: String) -> FrameGraph {
        let task = async move { assetio.load_frame_graph(descriptor).await };

        FrameGraph {
            graph: CompiledFrameGraph::Waiting(AsyncTask::start(task)),
        }
    }
}
