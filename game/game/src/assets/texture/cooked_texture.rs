use crate::assets::{AssetError, ImageDescriptor, ImageEncoding, SamplerDescriptor};
use image::ColorType;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct CookedTexture {
    pub data: Vec<u8>,
    pub image_descriptor: ImageDescriptor,
    pub sampler: SamplerDescriptor,
}

impl CookedTexture {
    pub fn decompress(mut self) -> Result<CookedTexture, AssetError> {
        log::info!("Texture image descriptor: {:#?}", self.image_descriptor);
        log::info!("Texture sampler descriptor: {:#?}", self.sampler);

        match self.image_descriptor.encoding {
            ImageEncoding::Png => {
                let img = image::load_from_memory(&self.data).unwrap();
                log::info!("Image color format: {:?}", img.color());
                self.data = match (img.color(), self.image_descriptor.format) {
                    (ColorType::Rgba8, wgpu::TextureFormat::Rgba8UnormSrgb) => Ok(img.as_rgba8().unwrap().to_vec()),
                    (ColorType::Rgba8, wgpu::TextureFormat::Rgba8Unorm) => Ok(img.as_rgba8().unwrap().to_vec()),
                    (c, f) => Err(AssetError::Content(format!(
                        "Unsupported image color({:?}) and texture format({:?})",
                        c, f
                    ))),
                }?;
                self.image_descriptor.encoding = ImageEncoding::Raw;
                Ok(self)
            }

            ImageEncoding::Jpeg => {
                let img = image::load_from_memory(&self.data).unwrap();
                log::info!("Image color format: {:?}", img.color());
                self.data = match (img.color(), self.image_descriptor.format) {
                    (ColorType::Rgb8, wgpu::TextureFormat::Rgba8UnormSrgb) => Ok(img.as_rgb8().unwrap().to_vec()),
                    (ColorType::Rgb8, wgpu::TextureFormat::Rgba8Unorm) => Ok(img.as_rgb8().unwrap().to_vec()),
                    (c, f) => Err(AssetError::Content(format!(
                        "Unsupported image color({:?}) and texture format({:?})",
                        c, f
                    ))),
                }?;
                self.image_descriptor.encoding = ImageEncoding::Raw;
                Ok(self)
            }

            ImageEncoding::Raw => unimplemented!(),
        }
    }
}
