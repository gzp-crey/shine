use crate::render::{RenderConfig, RenderError, Surface};
use std::sync::{Arc, Mutex};

/// Thread safe rendering context.
pub struct Context {
    //instance: wgpu::Instance,
    device: Arc<wgpu::Device>,
    queue: wgpu::Queue,
    commands: Mutex<Vec<wgpu::CommandBuffer>>,
    swap_chain_format: wgpu::TextureFormat,
    swap_chain: Option<(wgpu::SwapChain, wgpu::SwapChainDescriptor)>,
}

impl Context {
    pub async fn new(
        instance: wgpu::Instance,
        surface: &Surface,
        config: &RenderConfig,
    ) -> Result<Context, RenderError> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(surface.surface()),
            })
            .await
            .ok_or_else(|| RenderError::device_error_str("Adapter not found"))?;

        //log::info!("Graphics adapter: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                    shader_validation: config.enable_validation,
                },
                config.wgpu_trace.as_ref().map(std::path::Path::new),
            )
            .await
            .map_err(|err| RenderError::device_error("Failed to create device.", err))?;

        Ok(Context {
            //instance,
            device: Arc::new(device),
            queue,
            commands: Mutex::new(Vec::new()),
            swap_chain_format: config.swap_chain_format,
            swap_chain: None,
        })
    }

    pub fn device(&self) -> Arc<wgpu::Device> {
        self.device.clone()
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn clear_commands(&mut self) {
        let mut commands = self.commands.lock().unwrap();
        commands.clear();
    }

    pub fn add_command(&self, command: wgpu::CommandBuffer) {
        let mut commands = self.commands.lock().unwrap();
        commands.push(command);
    }

    pub fn submit_commands(&mut self) {
        let mut commands = self.commands.lock().unwrap();
        self.queue.submit(commands.drain(..));
    }

    pub fn swap_chain_format(&self) -> wgpu::TextureFormat {
        self.swap_chain_format
    }

    pub fn create_frame(
        &mut self,
        surface: &Surface,
    ) -> Result<(wgpu::SwapChainTexture, wgpu::SwapChainDescriptor), RenderError> {
        let device = &self.device;

        let format = self.swap_chain_format;
        let size = surface.size();

        if self
            .swap_chain
            .as_ref()
            .map(|(_, sd)| (sd.width, sd.height) != size)
            .unwrap_or(false)
        {
            self.swap_chain = None;
        };

        let (ref mut sc, sd) = self.swap_chain.get_or_insert_with(|| {
            let sd = wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format,
                width: surface.size().0,
                height: surface.size().1,
                present_mode: wgpu::PresentMode::Mailbox,
            };

            let sc = device.create_swap_chain(surface.surface(), &sd);
            (sc, sd)
        });

        let frame = sc
            .get_current_frame()
            .map_err(|err| RenderError::device_error("Failed to create frame", err))?
            .output;

        Ok((frame, sd.clone()))
    }
}
