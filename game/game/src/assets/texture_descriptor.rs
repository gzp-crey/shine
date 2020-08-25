use crate::assets::AssetError;
use image::ColorType;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU8;

/// The encoding for the texture image
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ImageEncoding {
    /// Image data is encoded as a png
    Png,

    /// Image data is encoded as a jpeg
    Jpeg,
}

/// Texture data descriptor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Image {
    pub encoding: ImageEncoding,
    pub format: wgpu::TextureFormat,
    pub size: (u32, u32),
}

impl Image {
    pub fn new() -> Image {
        Image {
            encoding: ImageEncoding::Png,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            size: (0, 0),
        }
    }
}

impl Default for Image {
    fn default() -> Self {
        Self::new()
    }
}

/// Sampler descriptor
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
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
    pub anisotropy_clamp: Option<NonZeroU8>,
    pub border_color: Option<wgpu::SamplerBorderColor>,
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
            border_color: None,
        }
    }
}

/// Texture and sampler descriptor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextureDescriptor {
    pub image: Image,
    pub sampler: SamplerDescriptor,
}

impl TextureDescriptor {
    pub fn new() -> TextureDescriptor {
        TextureDescriptor {
            image: Image::new(),
            sampler: SamplerDescriptor::new(),
        }
    }
}

impl Default for TextureDescriptor {
    fn default() -> Self {
        Self::new()
    }
}

/// Deserialized texture
#[derive(Clone, Serialize, Deserialize)]
pub struct TextureImage {
    pub data: Vec<u8>,
    pub image: Image,
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
}
