use crate::assets::AssetError;
use image::ColorType;
use serde::{Deserialize, Serialize};

/// The encoding for the texture image
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum TextureImageEncoding {
    Png,
    /*Dxt1,
    Dxt3,
    Dxt5,*/
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextureDescriptor {
    pub encoding: TextureImageEncoding,
    pub format: wgpu::TextureFormat,
    pub size: (u32, u32),

    pub address_mode_u: wgpu::AddressMode,
    pub address_mode_v: wgpu::AddressMode,
    pub address_mode_w: wgpu::AddressMode,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::FilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<wgpu::CompareFunction>,
    pub anisotropy_clamp: Option<u8>,
}

impl TextureDescriptor {
    pub fn new() -> TextureDescriptor {
        TextureDescriptor {
            encoding: TextureImageEncoding::Png,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            size: (0, 0),

            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: None,
            anisotropy_clamp: None,
        }
    }

    pub fn create_sampler_descriptor(&self) -> wgpu::SamplerDescriptor {
        wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: self.address_mode_u,
            address_mode_v: self.address_mode_v,
            address_mode_w: self.address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: self.mipmap_filter,
            lod_min_clamp: self.lod_min_clamp,
            lod_max_clamp: self.lod_max_clamp,
            compare: self.compare,
            anisotropy_clamp: self.anisotropy_clamp,
            ..Default::default()
        }
    }

    pub fn get_upload_info(&self) -> (u32, u32) {
        match self.format {
            wgpu::TextureFormat::Rgba8UnormSrgb => (4 * self.size.0, self.size.1),
            _ => unimplemented!(),
        }
    }
}

pub struct TextureBuffer {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TextureImage {
    pub image: Vec<u8>,
    pub descriptor: TextureDescriptor,
}

impl TextureImage {
    pub fn decompress(mut self) -> Result<TextureImage, AssetError> {
        log::info!("Texture descriptor: {:#?}", self.descriptor);
        match self.descriptor.encoding {
            TextureImageEncoding::Png => {
                let img = image::load_from_memory(&self.image).unwrap();
                log::info!("color: {:?}", img.color());
                self.image = match (img.color(), self.descriptor.format) {
                    (ColorType::Rgba8, wgpu::TextureFormat::Rgba8UnormSrgb) => Ok(img.as_rgba8().unwrap().to_vec()),
                    (ColorType::Rgba8, wgpu::TextureFormat::Rgba8Unorm) => Ok(img.as_rgba8().unwrap().to_vec()),
                    (c, f) => Err(AssetError::Content(format!(
                        "Unsupported image color({:?}) and texture format({:?})",
                        c, f
                    ))),
                }?;
                Ok(self)
            }
            _ => Ok(self),
        }
    }

    pub fn to_texture_buffer(&self, device: &wgpu::Device) -> Result<(TextureBuffer, wgpu::CommandBuffer), AssetError> {
        let (width, height) = self.descriptor.size;

        let size = wgpu::Extent3d {
            width,
            height,
            depth: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.descriptor.format,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        });

        let init_cmd_buffer = {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            let buffer = device.create_buffer_with_data(&self.image, wgpu::BufferUsage::COPY_SRC);
            let (bytes_per_row, rows_per_image) = self.descriptor.get_upload_info();
            encoder.copy_buffer_to_texture(
                wgpu::BufferCopyView {
                    buffer: &buffer,
                    layout: wgpu::TextureDataLayout {
                        offset: 0,
                        bytes_per_row,
                        rows_per_image,
                    },
                },
                wgpu::TextureCopyView {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                },
                size,
            );
            encoder.finish()
        };

        let view = texture.create_default_view();

        Ok((
            TextureBuffer {
                texture,
                view,
                sampler: device.create_sampler(&self.descriptor.create_sampler_descriptor()),
            },
            init_cmd_buffer,
        ))
    }
}
