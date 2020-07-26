use crate::assets::RenderTargetDescriptor;
use crate::render::Compile;

pub struct CompiledRenderTarget {
    pub texture: wgpu::Texture,
    pub size: (u32, u32),
}

pub struct RenderTargetCompileExtra {
    frame_size: (u32, u32),
    is_sampled: bool,
}

impl Compile<RenderTargetCompileExtra> for RenderTargetDescriptor {
    type Compiled = CompiledRenderTarget;

    fn compile(&self, device: &wgpu::Device, extra: RenderTargetCompileExtra) -> Self::Compiled {
        let size = self.get_target_size(extra.frame_size);

        let extent = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth: 1,
        };

        let usage = if extra.is_sampled {
            wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED
        } else {
            wgpu::TextureUsage::OUTPUT_ATTACHMENT
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage,
            label: None,
        });

        CompiledRenderTarget { texture, size }
    }
}
