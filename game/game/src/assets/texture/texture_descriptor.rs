use serde::{Deserialize, Serialize};
use std::num::NonZeroU8;

/// The encoding for the texture image. It could be quessed from the content
/// but it is much faster if we already store it and there is no need to guess.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum ImageEncoding {
    /// Image data is encoded as a png
    Png,

    /// Image data is encoded as a jpeg
    Jpeg,

    /// Raw uncompressed
    Raw,
}

/// Texture data descriptor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageDescriptor {
    pub encoding: ImageEncoding,
    pub format: wgpu::TextureFormat,
    pub size: (u32, u32),
}

impl Default for ImageDescriptor {
    fn default() -> Self {
        Self {
            encoding: ImageEncoding::Png,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            size: (0, 0),
        }
    }
}

/// Sampler descriptor
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
    pub anisotropy_clamp: Option<NonZeroU8>,
    pub border_color: Option<wgpu::SamplerBorderColor>,
}

impl Default for SamplerDescriptor {
    fn default() -> Self {
        Self {
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
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct TextureDescriptor {
    pub image: ImageDescriptor,
    pub sampler: SamplerDescriptor,
}
