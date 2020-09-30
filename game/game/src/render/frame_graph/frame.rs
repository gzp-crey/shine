use crate::{
    assets::{AssetError, AssetIO, FrameGraphDescriptor, FramePassDescriptor, RenderTargetDescriptor, Url},
    render::{
        frame_graph::{frame_pass::FramePasses, frame_target::FrameTargets, render_pass::RenderPass},
        Context, RenderError, Surface,
    },
};
use std::sync::Mutex;

pub struct FrameOutput {
    pub frame: wgpu::SwapChainTexture,
    pub descriptor: wgpu::SwapChainDescriptor,
}

pub struct Frame {
    targets: FrameTargets,
    passes: FramePasses,

    commands: Mutex<Vec<wgpu::CommandBuffer>>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            targets: Default::default(),
            passes: Default::default(),
            commands: Mutex::new(Vec::new()),
        }
    }
}

impl Frame {
    pub fn add_target(&mut self, name: String, target_descriptor: RenderTargetDescriptor) -> Result<(), RenderError> {
        self.targets.add_target(name, target_descriptor)?;
        Ok(())
    }
    pub fn remove_target(&mut self, target_name: &str) -> Result<(), RenderError> {
        self.targets.remove_target(target_name)?;
        Ok(())
    }

    pub fn add_pass(&mut self, name: String, pass_descriptor: FramePassDescriptor) -> Result<(), RenderError> {
        self.passes.add_pass(name, pass_descriptor)?;
        Ok(())
    }

    pub fn remove_pass(&mut self, pass_name: &str) -> Result<(), RenderError> {
        self.passes.remove_pass(pass_name)?;
        Ok(())
    }

    pub fn set_frame_graph(&mut self, graph: FrameGraphDescriptor) -> Result<(), RenderError> {
        self.passes.clear();
        self.targets.clear_targets();

        let FrameGraphDescriptor { targets, passes } = graph;
        for target in targets.into_iter() {
            self.add_target(target.0, target.1)?;
        }
        for pass in passes.into_iter() {
            self.add_pass(pass.0, pass.1)?;
        }

        Ok(())
    }

    fn try_resolve(&mut self, device: &wgpu::Device) -> Result<(), RenderError> {
        self.targets.resolve(device, &self.passes)?;
        self.passes.resolve(device, &self.targets)?;
        Ok(())
    }

    pub fn start_frame(&mut self, surface: &Surface, context: &mut Context) -> Result<(), RenderError> {
        let frame = context.create_frame(surface)?;
        self.targets.set_frame_output(Some(frame));

        if let Err(err) = self.try_resolve(context.device()) {
            log::warn!("Failed to resolve frame graph: {:?}", err);
            {
                // todo: is it ok to clear the render queue ???
                let mut commands = self.commands.lock().unwrap();
                commands.clear();
            }
            self.targets.set_frame_output(None);
            return Err(RenderError::GraphError);
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
        self.passes.begin_pass(encoder, &self.targets, pass_name)
    }

    pub fn add_command(&self, command: wgpu::CommandBuffer) {
        let mut commands = self.commands.lock().unwrap();
        commands.push(command);
    }
}

/// Error during frame graph load
#[derive(Debug)]
pub enum FrameGraphLoadError {
    Asset(AssetError),
    Canceled,
}

impl From<AssetError> for FrameGraphLoadError {
    fn from(err: AssetError) -> FrameGraphLoadError {
        FrameGraphLoadError::Asset(err)
    }
}

impl AssetIO {
    pub async fn load_frame_graph(&self, source_id: String) -> Result<FrameGraphDescriptor, FrameGraphLoadError> {
        log::debug!("[{:?}] Loading frame graph...", source_id);

        let url = Url::parse(&source_id).map_err(|err| AssetError::InvalidUrl(err))?;
        let data = self.download_binary(&url).await?;
        let descriptor = bincode::deserialize::<FrameGraphDescriptor>(&data)
            .map_err(|err| AssetError::load_failed(url.as_str(), err))?;
        log::trace!("Graph: {:#?}", descriptor);
        descriptor.check_target_references()?;

        Ok(descriptor)
    }
}
