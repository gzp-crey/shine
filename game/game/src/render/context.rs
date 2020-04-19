use crate::render::Surface;
use crate::wgpu;
use crate::GameError;

/// Thread safe rendering context.
pub struct Context {
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain: Option<(wgpu::SwapChain, (u32, u32))>,
}

impl Context {
    pub async fn new() -> Result<Context, GameError> {
        let adapter = wgpu::Adapter::request(
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
            .await;

        Ok(Context {
            device,
            queue,
            swap_chain: None,
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn init_swap_chain(&mut self, surface: &Surface) {
        let device = &self.device;
        if let Some((_, size)) = self.swap_chain {
            if size != *surface.size() {
                self.swap_chain = None
            }
        }

        let _ = self.swap_chain.get_or_insert_with(|| {
            let sc_desc = wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                width: surface.size().0,
                height: surface.size().1,
                present_mode: wgpu::PresentMode::Mailbox,
            };

            (device.create_swap_chain(surface.surface(), &sc_desc), *surface.size())
        });
    }
}
