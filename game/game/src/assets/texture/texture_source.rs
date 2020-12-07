#![cfg(feature = "cook")]
use crate::assets::{
    io::ContentHasher, AssetError, AssetIO, CookedTexture, CookingError, ImageEncoding, TextureDescriptor, Url,
};
use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageOutputFormat};
use tokio::task;

#[derive(Clone)]
pub struct TextureSource {
    pub source_url: Url,
    pub descriptor: TextureDescriptor,
    pub image: DynamicImage,
}

impl TextureSource {
    pub async fn load(io: &AssetIO, source_url: &Url) -> Result<(Self, String), AssetError> {
        log::debug!("[{}] Downloading texture...", source_url.as_str());
        let image_data = io.download_binary(&source_url).await?;

        let meta_url = source_url.set_extension("tex")?;
        log::debug!("[{}] Downloading (optional) descriptor...", meta_url.as_str());
        let meta_data = match io.download_binary(&meta_url).await {
            Ok(meta_data) => Some(meta_data),
            Err(AssetError::ContentSource { .. }) => {
                log::warn!("[{}] Missing texture descriptor", meta_url.as_str());
                None
            }
            Err(err) => return Err(err),
        };

        let source_hash = {
            let mut hasher = ContentHasher::new();
            hasher.add(&image_data);
            if let Some(meta_data) = &meta_data {
                hasher.add(&meta_data);
            }
            hasher.hash()
        };

        let descriptor = match meta_data {
            Some(meta) => serde_json::from_slice(&meta).map_err(|err| AssetError::ContentLoad {
                content: source_url.as_str().to_owned(),
                source: err.into(),
            })?,
            None => TextureDescriptor::default(),
        };

        log::debug!("[{}] Docompressing image...", source_url.as_str());
        let image = task::spawn_blocking(move || image::load_from_memory(&image_data))
            .await
            .map_err(|err| AssetError::load_failed(source_url.as_str(), err))?
            .map_err(|err| AssetError::load_failed(source_url.as_str(), err))?;

        let texture_source = TextureSource {
            source_url: source_url.clone(),
            descriptor,
            image,
        };

        Ok((texture_source, source_hash))
    }

    pub async fn cook(self) -> Result<CookedTexture, CookingError> {
        log::debug!("[{}] Compiling...", self.source_url.as_str());
        log::trace!(
            "[{}] TextureDescriptor: \n{:#?}",
            self.source_url.as_str(),
            self.descriptor
        );

        let TextureSource {
            source_url,
            mut image,
            mut descriptor,
        } = self;

        if descriptor.image.size != (0, 0) {
            let (w, h) = descriptor.image.size;
            log::debug!("[{}] Resizing texture to ({},{})...", source_url.as_str(), w, h);
            image = task::spawn_blocking(move || image.resize_exact(w, h, FilterType::CatmullRom))
                .await
                .map_err(|err| CookingError::new(source_url.as_str(), err))?;
        } else {
            let (w, h) = image.dimensions();
            log::debug!("[{}] Updating descriptor size to ({},{})...", source_url.as_str(), w, h);
            descriptor.image.size = image.dimensions();
        }

        log::debug!("[{}] Recompressing texture...", source_url.as_str());
        let encoding = descriptor.image.encoding;
        let data = task::spawn_blocking({
            let source_url = source_url.clone();
            move || match encoding {
                ImageEncoding::Png => {
                    let mut image_data = Vec::new();
                    image
                        .write_to(&mut image_data, ImageOutputFormat::Png)
                        .map_err(|err| CookingError::new(source_url.as_str(), err))?;
                    Ok::<_, CookingError>(image_data)
                }
                ImageEncoding::Jpeg => {
                    let mut image_data = Vec::new();
                    image
                        .write_to(&mut image_data, ImageOutputFormat::Jpeg(80))
                        .map_err(|err| CookingError::new(source_url.as_str(), err))?;
                    Ok::<_, CookingError>(image_data)
                }

                ImageEncoding::Raw => unimplemented!(),
            }
        })
        .await
        .map_err(|err| CookingError::new(source_url.as_str(), err))??;

        Ok(CookedTexture {
            data,
            image_descriptor: descriptor.image,
            sampler: descriptor.sampler,
        })
    }
}
