use crate::assets::{AssetError, Image, RenderTargetDescriptor, RenderTargetSize, SamplerDescriptor, TextureImage};
use crate::render::Compile;

fn create_sampler_descriptor(descriptor: &SamplerDescriptor) -> wgpu::SamplerDescriptor {
    wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: descriptor.address_mode_u,
        address_mode_v: descriptor.address_mode_v,
        address_mode_w: descriptor.address_mode_w,
        mag_filter: descriptor.mag_filter,
        min_filter: descriptor.min_filter,
        mipmap_filter: descriptor.mipmap_filter,
        lod_min_clamp: descriptor.lod_min_clamp,
        lod_max_clamp: descriptor.lod_max_clamp,
        compare: descriptor.compare,
        anisotropy_clamp: descriptor.anisotropy_clamp,
    }
}

fn get_texture_data_layout(descriptor: &Image) -> (wgpu::Extent3d, wgpu::TextureDataLayout) {
    let size = wgpu::Extent3d {
        width: descriptor.size.0,
        height: descriptor.size.1,
        depth: 1,
    };

    let layout = match descriptor.format {
        wgpu::TextureFormat::Rgba8UnormSrgb => wgpu::TextureDataLayout {
            offset: 0,
            bytes_per_row: 4 * descriptor.size.0,
            rows_per_image: descriptor.size.1,
        },
        _ => unimplemented!(),
    };

    (size, layout)
}

impl Compile<()> for Image {
    type Compiled = Result<wgpu::Texture, AssetError>;

    fn compile(&self, device: &wgpu::Device, _extra: ()) -> Self::Compiled {
        let size = wgpu::Extent3d {
            width: self.size.0,
            height: self.size.1,
            depth: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        Ok(texture)
    }
}

/// Compiled texture and sampler
pub struct CompiledTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Compile<()> for TextureImage {
    type Compiled = Result<(CompiledTexture, Option<wgpu::CommandBuffer>), AssetError>;

    fn compile(&self, device: &wgpu::Device, _extra: ()) -> Self::Compiled {
        let texture = self.image.compile(device, ())?;

        let init_cmd_buffer = if !self.data.is_empty() {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            let buffer = device.create_buffer_with_data(&self.data, wgpu::BufferUsage::COPY_SRC);
            let (size, texture_data_layout) = get_texture_data_layout(&self.image);
            encoder.copy_buffer_to_texture(
                wgpu::BufferCopyView {
                    buffer: &buffer,
                    layout: texture_data_layout,
                },
                wgpu::TextureCopyView {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                size,
            );
            Some(encoder.finish())
        } else {
            None
        };

        let sampler = device.create_sampler(&create_sampler_descriptor(&self.sampler));
        let view = texture.create_default_view();

        Ok((CompiledTexture { texture, view, sampler }, init_cmd_buffer))
    }
}

pub struct CompiledRenderTarget {
    pub texture: wgpu::Texture,
    pub size: (u32, u32),
    //pub view: wgpu::TextureView,
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

        Ok(CompiledRenderTarget { texture, size })
    }
}
