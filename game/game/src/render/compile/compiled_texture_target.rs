use crate::assets::TextureTargetDescriptor;
use crate::render::Compile;

pub struct CompiledTextureTarget {
    pub format: wgpu::TextureFormat,
    pub size: (u32, u32),
    pub is_sampled: bool,

    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

pub struct TextureTargetCompileExtra {
    pub frame_size: (u32, u32),
}

impl Compile<TextureTargetCompileExtra> for TextureTargetDescriptor {
    type Compiled = CompiledTextureTarget;

    fn compile(&self, device: &wgpu::Device, extra: TextureTargetCompileExtra) -> Self::Compiled {
        let size = self.get_target_size(extra.frame_size);

        let extent = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth: 1,
        };

        let usage = if self.is_sampled {
            wgpu::TextureUsage::OUTPUT_ATTACHMENT | wgpu::TextureUsage::SAMPLED
        } else {
            wgpu::TextureUsage::OUTPUT_ATTACHMENT
        };

        log::warn!("render target texture usage: {:?}", usage);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage,
            label: None,
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        CompiledTextureTarget {
            format: self.format,
            is_sampled: self.is_sampled,
            size,
            texture,
            view,
        }
    }
}
