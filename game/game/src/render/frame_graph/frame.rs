use crate::{
    assets::{AssetError, AssetIO, FrameGraphDescriptor, Url, UrlError},
    render::{
        frame_graph::{frame_target::FrameTargets, pass::Pass, render_pass::RenderPass},
        Context, RenderError, Surface,
    },
};
use shine_ecs::core::async_task::AsyncTask;
use std::sync::Mutex;

pub struct FrameOutput {
    pub frame: wgpu::SwapChainTexture,
    pub descriptor: wgpu::SwapChainDescriptor,
}

#[derive(Debug, Clone)]
pub struct FrameGraphError;

pub struct Frame {
    descriptor: Option<Result<FrameGraphDescriptor, FrameGraphLoadError>>,
    descriptor_loader: Option<AsyncTask<Result<FrameGraphDescriptor, FrameGraphLoadError>>>,

    targets: FrameTargets,
    passes: Result<Vec<Pass>, FrameGraphError>,

    commands: Mutex<Vec<wgpu::CommandBuffer>>,
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            descriptor: Some(Ok(FrameGraphDescriptor::single_pass())),
            descriptor_loader: None,
            targets: FrameTargets::new(),
            passes: Ok(Vec::new()),
            commands: Mutex::new(Vec::new()),
        }
    }

    pub fn load_graph(&mut self, assetio: AssetIO, descriptor: String) {
        self.passes = Ok(Vec::new());
        self.targets.clear_targets();
        let task = async move { assetio.load_frame_graph(descriptor).await };
        self.descriptor_loader = Some(AsyncTask::start(task));
        self.descriptor = None;
    }

    pub fn set_graph(&mut self, descriptor: Result<FrameGraphDescriptor, FrameGraphLoadError>) {
        self.passes = Ok(Vec::new());
        self.targets.clear_targets();
        self.descriptor_loader = None;
        self.descriptor = Some(descriptor);
    }

    fn recompile_frame_graph(&mut self, context: &Context) -> Result<(), FrameGraphError> {
        log::info!("Compiling frame graph");
        match &self.descriptor {
            Some(Ok(desc)) => {
                log::debug!("Creating render targets");
                self.targets.recompile_targets(context.device(), desc)?;

                log::debug!("Creating passes");
                let mut passes = Vec::new();
                for (name, pass_desc) in &desc.passes {
                    log::trace!("Creating pass {}", name);
                    let pass = Pass::create(context.device(), &self.targets, name, pass_desc)?;
                    passes.push(pass);
                }
                self.passes = Ok(passes);
                Ok(())
            }
            Some(Err(_)) => Err(FrameGraphError),
            None => {
                // for explicit setting, descriptor is always some
                // for async loaded, recompile should not be called until desc's been loaded
                unreachable!("Missing frame graph descriptor");
            }
        }
    }

    fn try_get_new_descriptor(&mut self) -> Option<Result<FrameGraphDescriptor, FrameGraphLoadError>> {
        if let Some(loader) = self.descriptor_loader.as_mut() {
            log::error!("Checking loader for frame graph");
            match loader.try_get() {
                Err(_) => {
                    log::warn!("Frame graph load canceled");
                    Some(Err(FrameGraphLoadError::Canceled))
                }
                Ok(Some(Err(err))) => {
                    log::warn!("Frame graph load failed: {:?}", err);
                    Some(Err(err))
                }
                Ok(Some(Ok(descriptor))) => Some(Ok(descriptor)),
                Ok(None) => None,
            }
        } else {
            None
        }
    }

    pub fn start_frame(&mut self, surface: &Surface, context: &mut Context) -> Result<(), RenderError> {
        if let Some(descriptor) = self.try_get_new_descriptor() {
            self.set_graph(descriptor);
        }
        if self.descriptor.is_none() {
            return Err(RenderError::GraphNotReady);
        }
        if self.passes.is_err() {
            return Err(RenderError::GraphError);
        }

        let frame = context.create_frame(surface)?;
        self.targets.set_frame_output(Some(frame));

        if self.passes.as_ref().map(|p| p.is_empty()).unwrap() {
            if let Err(err) = self.recompile_frame_graph(context) {
                log::warn!("Failed to compile frame graph: {:?}", err);
                {
                    // todo: is it ok to clear the render queue ???
                    let mut commands = self.commands.lock().unwrap();
                    commands.clear();
                }
                self.targets.clear_targets();
                self.targets.set_frame_output(None);
                self.passes = Err(err);
                return Err(RenderError::GraphError);
            }
        } else {
            // check frame size, resize targets
        }

        Ok(())
    }

    pub fn end_frame(&mut self, queue: &wgpu::Queue) {
        {
            let mut commands = self.commands.lock().unwrap();
            queue.submit(commands.drain(..));
        }
        self.targets.set_frame_output(None);
    }

    pub fn frame_output(&self) -> Option<&FrameOutput> {
        self.targets.frame_output()
    }

    pub fn frame_size(&self) -> (u32, u32) {
        self.targets.frame_size()
    }

    pub fn begin_pass<'r, 'f: 'r, 'e: 'f>(
        &'f self,
        encoder: &'e mut wgpu::CommandEncoder,
        pass_name: &'f str,
    ) -> Result<RenderPass<'r>, RenderError> {
        if let Ok(passes) = &self.passes {
            if let Some(pass) = passes.iter().find(|x| x.name == pass_name) {
                Ok(RenderPass::new(pass, &self.targets, &self.commands, encoder))
            } else {
                //log::warn!("No [{}] pass was found", pass);
                Err(RenderError::MissingPass(pass_name.to_owned()))
            }
        } else {
            Err(RenderError::GraphError)
        }
    }

    pub fn add_command(&self, command: wgpu::CommandBuffer) {
        let mut commands = self.commands.lock().unwrap();
        commands.push(command);
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
        descriptor.check_target_references()?;

        Ok(descriptor)
    }
}
