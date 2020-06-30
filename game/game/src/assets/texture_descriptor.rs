use crate::assets::AssetError;
use image::ColorType;
use serde::{Deserialize, Serialize};

/// The encoding for the texture image
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ImageEncoding {
    /// Image data is encoded as a png
    Png,

    /// Image data is encoded as a jpeg
    Jpeg,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageDescriptor {
    pub encoding: ImageEncoding,
    pub format: wgpu::TextureFormat,
    pub size: (u32, u32),
}

impl ImageDescriptor {
    pub fn new() -> ImageDescriptor {
        ImageDescriptor {
            encoding: ImageEncoding::Png,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            size: (0, 0),
        }
    }

    pub fn get_texture_data_layout(&self) -> (wgpu::Extent3d, wgpu::TextureDataLayout) {
        let size = wgpu::Extent3d {
            width: self.size.0,
            height: self.size.1,
            depth: 1,
        };

        let layout = match self.format {
            wgpu::TextureFormat::Rgba8UnormSrgb => wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * self.size.0,
                rows_per_image: self.size.1,
            },
            _ => unimplemented!(),
        };

        (size, layout)
    }

    pub fn to_texture(&self, device: &wgpu::Device) -> Result<(wgpu::Texture, wgpu::TextureView), AssetError> {
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
        let view = texture.create_default_view();

        Ok((texture, view))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SamplerDescriptor {
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

impl SamplerDescriptor {
    pub fn new() -> SamplerDescriptor {
        SamplerDescriptor {
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextureDescriptor {
    pub image: ImageDescriptor,
    pub sampler: SamplerDescriptor,
}

impl TextureDescriptor {
    pub fn new() -> TextureDescriptor {
        TextureDescriptor {
            image: ImageDescriptor::new(),
            sampler: SamplerDescriptor::new(),
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
    pub data: Vec<u8>,
    pub image: ImageDescriptor,
    pub sampler: SamplerDescriptor,
}

impl TextureImage {
    pub fn decompress(mut self) -> Result<TextureImage, AssetError> {
        log::info!("Texture image descriptor: {:#?}", self.image);
        log::info!("Texture sampler descriptor: {:#?}", self.sampler);
        match self.image.encoding {
            ImageEncoding::Png => {
                let img = image::load_from_memory(&self.data).unwrap();
                log::info!("Image color format: {:?}", img.color());
                self.data = match (img.color(), self.image.format) {
                    (ColorType::Rgba8, wgpu::TextureFormat::Rgba8UnormSrgb) => Ok(img.as_rgba8().unwrap().to_vec()),
                    (ColorType::Rgba8, wgpu::TextureFormat::Rgba8Unorm) => Ok(img.as_rgba8().unwrap().to_vec()),
                    (c, f) => Err(AssetError::Content(format!(
                        "Unsupported image color({:?}) and texture format({:?})",
                        c, f
                    ))),
                }?;
                Ok(self)
            }

            ImageEncoding::Jpeg => {
                let img = image::load_from_memory(&self.data).unwrap();
                log::info!("Image color format: {:?}", img.color());
                self.data = match (img.color(), self.image.format) {
                    (ColorType::Rgb8, wgpu::TextureFormat::Rgba8UnormSrgb) => Ok(img.as_rgb8().unwrap().to_vec()),
                    (ColorType::Rgb8, wgpu::TextureFormat::Rgba8Unorm) => Ok(img.as_rgb8().unwrap().to_vec()),
                    (c, f) => Err(AssetError::Content(format!(
                        "Unsupported image color({:?}) and texture format({:?})",
                        c, f
                    ))),
                }?;
                Ok(self)
            }
        }
    }

    pub fn to_texture_buffer(
        &self,
        device: &wgpu::Device,
    ) -> Result<(TextureBuffer, Option<wgpu::CommandBuffer>), AssetError> {
        let (texture, view) = self.image.to_texture(device)?;

        let init_cmd_buffer = if !self.data.is_empty() {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            let buffer = device.create_buffer_with_data(&self.data, wgpu::BufferUsage::COPY_SRC);
            let (size, texture_data_layout) = self.image.get_texture_data_layout();
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

        let sampler = device.create_sampler(&self.sampler.create_sampler_descriptor());

        Ok((TextureBuffer { texture, view, sampler }, init_cmd_buffer))
    }
}
