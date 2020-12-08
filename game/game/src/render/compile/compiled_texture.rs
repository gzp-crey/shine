use crate::{
    assets::{CookedTexture, ImageDescriptor, ImageEncoding, SamplerDescriptor},
    render::{Compile, RenderError},
};
use wgpu::util::DeviceExt;

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

impl<'a> Compile for &'a SamplerDescriptor {
    type Output = wgpu::Sampler;

    fn compile(self, device: &wgpu::Device) -> Self::Output {
        device.create_sampler(&create_sampler_descriptor(self))
    }
}

fn get_texture_data_layout(descriptor: &ImageDescriptor) -> (wgpu::Extent3d, wgpu::TextureDataLayout) {
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

impl<'a> Compile for &'a ImageDescriptor {
    type Output = Result<wgpu::Texture, RenderError>;

    fn compile(self, device: &wgpu::Device) -> Self::Output {
        if self.encoding != ImageEncoding::Raw {
            return Err(RenderError::Compile {
                message: format!("Compressed texture format is not supported ({:?})", self.encoding),
            });
        }

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

impl<'a> Compile for &'a CookedTexture {
    type Output = Result<(CompiledTexture, Option<wgpu::CommandBuffer>), RenderError>;

    fn compile(self, device: &wgpu::Device) -> Self::Output {
        let texture = self.image_descriptor.compile(device)?;

        let init_cmd_buffer = if !self.data.is_empty() {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &self.data,
                usage: wgpu::BufferUsage::COPY_SRC,
            });
            let (size, texture_data_layout) = get_texture_data_layout(&self.image_descriptor);
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

        let sampler = self.sampler.compile(device);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Ok((CompiledTexture { texture, view, sampler }, init_cmd_buffer))
    }
}
