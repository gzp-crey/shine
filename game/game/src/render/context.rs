use crate::render::{FrameOutput, Surface};
use crate::wgpu;
use crate::GameError;

/// Thread safe rendering context.
pub struct Context {
    //instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain_format: wgpu::TextureFormat,
    swap_chain: Option<(wgpu::SwapChain, wgpu::SwapChainDescriptor)>,
}

//https://github.com/gfx-rs/wgpu-rs/issues/287
#[cfg(feature = "wasm")]
mod wasm_hack {
    unsafe impl Send for super::Context {}
    unsafe impl Sync for super::Context {}
}

impl Context {
    pub async fn new(instance: wgpu::Instance) -> Result<Context, GameError> {
        let adapter = instance
            .request_adapter(
                &wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::Default,
                    compatible_surface: None,
                },
                wgpu::BackendBit::PRIMARY,
            )
            .await
            .ok_or(GameError::RenderContext("Adapter not found".to_owned()))?;

        //log::info!("Graphics adapter: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                extensions: wgpu::Extensions {
                    anisotropic_filtering: false,
                },
                limits: wgpu::Limits::default(),
            })
            .await
            .map_err(|err| GameError::RenderContext(format!("Failed to create device: {:?}", err)))?;

        Ok(Context {
            //instance,
            device,
            queue,
            swap_chain_format: wgpu::TextureFormat::Bgra8UnormSrgb,
            swap_chain: None,
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn swap_chain_format(&self) -> wgpu::TextureFormat {
        self.swap_chain_format
    }

    pub fn create_frame(&mut self, surface: &Surface) -> Result<FrameOutput, String> {
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
            .get_next_texture()
            .map_err(|err| format!("Frame request error: {:?}", err))?;
        Ok(FrameOutput {
            frame,
            descriptor: sd.clone(),
        })
    }
}
