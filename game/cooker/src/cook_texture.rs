use crate::{Context, CookingError};
use image::{dxt, imageops::FilterType, DynamicImage, GenericImageView, ImageError, ImageOutputFormat};
use shine_game::assets::{AssetError, AssetIO, TextureDescriptor, TextureImage, TextureImageEncoding, Url};
use shine_game::wgpu;
use tokio::task;

impl From<ImageError> for CookingError {
    fn from(err: ImageError) -> CookingError {
        AssetError::Content(format!("Image error: {}", err)).into()
    }
}

pub async fn load_image(assetio: &AssetIO, image_url: &Url) -> Result<DynamicImage, CookingError> {
    log::debug!("[{}] Downloading image...", image_url.as_str());
    let data = assetio.download_binary(&image_url).await?;

    log::debug!("[{}] Docompressing image...", image_url.as_str());
    let image = task::spawn_blocking(move || image::load_from_memory(&data)).await??;

    log::trace!(
        "[{}] Image:\n  size: {:?}\n  color: {:?}",
        image_url.as_str(),
        image.dimensions(),
        image.color()
    );
    Ok(image)
}

pub async fn load_descriptor(assetio: &AssetIO, meta_url: &Url) -> Result<TextureDescriptor, CookingError> {
    log::debug!("[{}] Downloading descriptor...", meta_url.as_str());
    match assetio.download_string(&meta_url).await {
        Ok(data) => Ok(serde_json::from_str(&data)?),
        Err(AssetError::AssetProvider(_)) => {
            log::warn!("[{}] Missing  texture descriptor", meta_url.as_str());
            Ok(TextureDescriptor::new())
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn cook_texture(
    context: &Context,
    _source_base: &Url,
    target_base: &Url,
    texture_url: &Url,
) -> Result<String, CookingError> {
    let mut image = load_image(&context.assetio, texture_url).await?;
    let mut descriptor = load_descriptor(&context.assetio, &texture_url.set_extension("tex")?).await?;

    if descriptor.size != (0, 0) {
        let (w, h) = descriptor.size;
        log::debug!("[{}] Resizing texture to ({},{})...", texture_url.as_str(), w, h);
        image = task::spawn_blocking(move || image.resize_exact(w, h, FilterType::CatmullRom)).await?;
    } else {
        descriptor.size = image.dimensions();
    }

    log::debug!(
        "[{}] Converting color space for texture format {:?}...",
        texture_url.as_str(),
        descriptor.format
    );
    let format = descriptor.format;
    image = task::spawn_blocking(move || match format {
        wgpu::TextureFormat::Rgba8UnormSrgb => Ok(DynamicImage::ImageRgba8(image.into_rgba())),
        wgpu::TextureFormat::Rgba8Unorm => Ok(DynamicImage::ImageRgba8(image.into_rgba())),
        f => Err(AssetError::Content(format!("Unsupported texture format({:?}) ", f))),
    })
    .await??;

    //todo: reshape image the match format
    log::trace!("[{}] Texture descriptor:\n{:#?}", texture_url.as_str(), descriptor);

    log::debug!("[{}] Compressing texture...", texture_url.as_str());
    let encoding = descriptor.encoding;
    let image = task::spawn_blocking(move || match encoding {
        TextureImageEncoding::Png => {
            let mut image_data = Vec::new();
            image.write_to(&mut image_data, ImageOutputFormat::Png)?;
            Ok::<_, CookingError>(image_data)
        }
    })
    .await??;
    log::trace!(
        "[{}] Cooked texture descriptor:\n{:#?}",
        texture_url.as_str(),
        descriptor
    );

    log::debug!("[{}] Uploading...", texture_url.as_str());
    let cooked_texture = bincode::serialize(&TextureImage { descriptor, image })?;
    let target_id = context
        .assetio
        .upload_cooked_binary(&target_base, "tex", &cooked_texture)
        .await?;
    Ok(target_id)
}
