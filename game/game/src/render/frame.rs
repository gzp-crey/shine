use crate::{
    assets::{AssetError, AssetIO, FrameGraphDescriptor, Url, UrlError},
    render::CompiledRenderTarget,
};
use shine_ecs::core::async_task::AsyncTask;
use std::{borrow::Cow, sync::Mutex};

struct Pass {
    name: String,
}

pub struct FrameOutput {
    pub frame: wgpu::SwapChainTexture,
    pub descriptor: wgpu::SwapChainDescriptor,
}

pub struct FrameTarget {
    name: String,
    render_target: CompiledRenderTarget,
}

pub struct FrameTextures<'t> {
    textures: Vec<&'t FrameTarget>,
}

#[derive(Debug, Clone)]
pub struct FrameGraphError;

pub struct Frame {
    descriptor: Result<Option<FrameGraphDescriptor>, FrameGraphError>,
    descriptor_loader: Option<AsyncTask<Result<FrameGraphDescriptor, FrameGraphLoadError>>>,

    frame_output: Option<FrameOutput>,
    targets: Vec<FrameTarget>,
    passes: Vec<Pass>,

    buffers: Mutex<Vec<wgpu::CommandBuffer>>,
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            descriptor: Ok(None),
            descriptor_loader: None,
            frame_output: None,
            targets: Vec::new(),
            passes: Vec::new(),
            buffers: Mutex::new(Vec::new()),
        }
    }

    pub fn load_graph(&mut self, assetio: AssetIO, descriptor: String) {
        let task = async move { assetio.load_frame_graph(descriptor).await };
        self.descriptor_loader = Some(AsyncTask::start(task));
    }

    pub fn set_graph(&mut self, descriptor: Option<FrameGraphDescriptor>) {
        self.set_frame_graph(Ok(descriptor));
    }

    fn set_frame_graph(&mut self, descriptor: Result<Option<FrameGraphDescriptor>, FrameGraphError>) {
        self.descriptor_loader = None;
        self.descriptor = descriptor;
        log::info!("rebuilding frame graph");
        //todo: create targets
    }

    pub fn start_frame(&mut self, frame: FrameOutput) {
        self.frame_output = Some(frame);

        if let Some(loader) = self.descriptor_loader.as_mut() {
            match loader.try_get() {
                Err(_) => self.set_frame_graph(Err(FrameGraphError)),
                Ok(Some(Ok(descriptor))) => self.set_frame_graph(Ok(Some(descriptor))),
                Ok(Some(Err(err))) => {
                    log::warn!("Frame graph load failed: {:?}", err);
                    self.set_frame_graph(Err(FrameGraphError));
                }
                Ok(None) => {}
            }
        };

        // check frame size, resize targets
    }

    pub fn postprocess_frame(&mut self) {}

    pub fn end_frame(&mut self, queue: &wgpu::Queue) {
        {
            let mut buffers = self.buffers.lock().unwrap();
            queue.submit(buffers.drain(..));
        }
        self.frame_output = None;
    }

    pub fn frame_output(&self) -> Option<&FrameOutput> {
        self.frame_output.as_ref()
    }

    pub fn create_pass<'e>(
        &'e self,
        encoder: &'e mut wgpu::CommandEncoder,
        pass: &str,
    ) -> (wgpu::RenderPass<'e>, FrameTextures<'e>) {
        if pass == "DEBUG" {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: Cow::Borrowed(&[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &self.frame_output().unwrap().frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: true,
                    },
                }]),
                depth_stencil_attachment: None,
            });

            let textures = FrameTextures { textures: Vec::new() };

            (pass, textures)
        } else {
            unimplemented!()
        }
    }

    pub fn add_command(&self, commands: wgpu::CommandBuffer) {
        let mut buffers = self.buffers.lock().unwrap();
        buffers.push(commands);
    }
}

impl Default for Frame {
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
