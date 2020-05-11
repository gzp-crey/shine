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
    pub size: (u32, u32),

    pub address_mode_u: wgpu::AddressMode,
    pub address_mode_v: wgpu::AddressMode,
    pub address_mode_w: wgpu::AddressMode,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::FilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: wgpu::CompareFunction,
}

impl TextureDescriptor {
    pub fn new() -> TextureDescriptor {
        TextureDescriptor {
            encoding: TextureImageEncoding::Png,
            size: (0, 0),

            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            compare: wgpu::CompareFunction::Undefined,
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
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TextureImage {
    pub image: Vec<u8>,
    pub descriptor: TextureDescriptor,
}
