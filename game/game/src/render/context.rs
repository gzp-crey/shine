use crate::GameError;
use wgpu;

pub struct Context {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain: Option<wgpu::SwapChain>,
}

impl Context {
    pub async fn new(surface: wgpu::Surface) -> Result<Context, GameError> {
        let adapter = wgpu::Adapter::request(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
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
            surface,
            device,
            queue,
            swap_chain: None,
        })
    }

    fn swap_chain(&mut self, size: (u32, u32)) -> &mut wgpu::SwapChain {
        let surface = &self.surface;
        let device = &self.device;
        self.swap_chain.get_or_insert_with(|| {
            let sc_desc = wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                width: size.0,
                height: size.1,
                present_mode: wgpu::PresentMode::Mailbox,
            };

            device.create_swap_chain(surface, &sc_desc)
        })
    }
}
