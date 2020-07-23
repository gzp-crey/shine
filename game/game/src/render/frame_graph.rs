use crate::assets::{AssetError, AssetIO, FrameGraphDescriptor, Url, UrlError};
use crate::render::{FrameOutput, PipelineStore, PipelineStoreRead};
use shine_ecs::core::async_task::AsyncTask;

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
    frame_output: Option<FrameOutput>,
    targets: Vec<RenderTarget>,
    passes: Vec<Pass>,
}

impl FrameGraphBuffer {
    pub fn new() -> FrameGraphBuffer {
        FrameGraphBuffer {
            frame_output: None,
            targets: Vec::new(),
            passes: Vec::new(),
        }
    }

    pub fn from_descriptor(_descriptor: FrameGraphDescriptor, frame_output: Option<FrameOutput>) -> FrameGraphBuffer {
        FrameGraphBuffer {
            frame_output,
            targets: Vec::new(),
            passes: Vec::new(),
        }
    }
}

impl Default for FrameGraphBuffer {
    fn default() -> Self {
        Self::new()
    }
}

enum CompiledFrameGraph {
    Error(Option<FrameOutput>),
    Waiting(
        Option<FrameOutput>,
        AsyncTask<Result<FrameGraphDescriptor, FrameGraphLoadError>>,
    ),
    Compiled(FrameGraphBuffer),
}

impl CompiledFrameGraph {
    fn frame_output(&self) -> Option<&FrameOutput> {
        match self {
            CompiledFrameGraph::Error(fr) => fr.as_ref(),
            CompiledFrameGraph::Waiting(fr, _) => fr.as_ref(),
            CompiledFrameGraph::Compiled(graph) => graph.frame_output.as_ref(),
        }
    }

    fn start_frame(&mut self, frame: Option<FrameOutput>) {
        match self {
            CompiledFrameGraph::Waiting(_, recv) => match recv.try_get() {
                Err(_) => *self = CompiledFrameGraph::Error(frame),
                Ok(None) => {}
                Ok(Some(Err(_))) => *self = CompiledFrameGraph::Error(frame),
                Ok(Some(Ok(descriptor))) => {
                    *self = CompiledFrameGraph::Compiled(FrameGraphBuffer::from_descriptor(descriptor, frame))
                }
            },
            CompiledFrameGraph::Error(fr) => *fr = frame,
            CompiledFrameGraph::Compiled(graph) => graph.frame_output = frame,
        };
    }

    fn end_frame(&mut self) {
        match self {
            CompiledFrameGraph::Waiting(fr, _) => *fr = None,
            CompiledFrameGraph::Error(fr) => *fr = None,
            CompiledFrameGraph::Compiled(graph) => graph.frame_output = None,
        };
    }
}

pub struct FrameGraph {
    graph: CompiledFrameGraph,
}

impl FrameGraph {
    pub fn new() -> FrameGraph {
        let graph = FrameGraphBuffer::new();
        FrameGraph {
            graph: CompiledFrameGraph::Compiled(graph),
        }
    }

    pub fn from_descriptor(descriptor: FrameGraphDescriptor) -> FrameGraph {
        let graph = FrameGraphBuffer::from_descriptor(descriptor, None);
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

    pub fn start_frame(&mut self, frame: Option<FrameOutput>) {
        self.graph.start_frame(frame);
    }

    pub fn frame_output(&self) -> Option<&FrameOutput> {
        self.graph.frame_output()
    }

    pub fn end_frame(&mut self) {
        self.graph.end_frame();
    }
}

impl Default for FrameGraph {
    fn default() -> Self {
        Self::new()
    }
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
            graph: CompiledFrameGraph::Waiting(None, AsyncTask::start(task)),
        }
    }
}
