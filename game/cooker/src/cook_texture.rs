use crate::{AssetNaming, Context, CookingError, TargetDependency};
use image::{dxt, imageops::FilterType, DynamicImage, GenericImageView, ImageError, ImageOutputFormat};
use shine_game::assets::{AssetError, TextureDescriptor, TextureImage, TextureImageEncoding, Url};
use shine_game::wgpu;
use tokio::task;

impl From<ImageError> for CookingError {
    fn from(err: ImageError) -> CookingError {
        AssetError::Content(format!("Image error: {}", err)).into()
    }
}

async fn load_etag(context: &Context, texture_url: &Url) -> Result<(String, Option<String>), CookingError> {
    log::debug!("[{}] Downloading texture...", texture_url.as_str());
    let image_etag = context.source_io.download_etag(&texture_url).await?;

    let meta_url = texture_url.set_extension("tex")?;
    log::debug!("[{}] Downloading descriptor...", meta_url.as_str());
    match context.source_io.download_etag(&meta_url).await {
        Ok(meta_etag) => Ok((image_etag, Some(meta_etag))),
        Err(AssetError::AssetProvider(_)) => {
            log::warn!("[{}] Missing  texture descriptor", meta_url.as_str());
            Ok((image_etag, None))
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn get_texture_etag(context: &Context, texture_url: &Url) -> Result<String, CookingError> {
    match load_etag(context, texture_url).await? {
        (img, Some(meta)) => Ok(format!("{},{}", img, meta)),
        (img, None) => Ok(img),
    }
}

async fn load_data(context: &Context, texture_url: &Url) -> Result<(Vec<u8>, Option<Vec<u8>>), CookingError> {
    log::debug!("[{}] Downloading texture...", texture_url.as_str());
    let image_data = context.source_io.download_binary(&texture_url).await?;

    let meta_url = texture_url.set_extension("tex")?;
    log::debug!("[{}] Downloading descriptor...", meta_url.as_str());
    match context.source_io.download_binary(&meta_url).await {
        Ok(meta_data) => Ok((image_data, Some(meta_data))),
        Err(AssetError::AssetProvider(_)) => {
            log::warn!("[{}] Missing  texture descriptor", meta_url.as_str());
            Ok((image_data, None))
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn cook_texture(
    context: &Context,
    asset_base: &Url,
    texture_url: &Url,
) -> Result<TargetDependency, CookingError> {
    log::debug!("[{}] Cooking...", texture_url.as_str());
    let source_hash = get_texture_etag(context, &texture_url).await?;

    log::debug!("[{}] Downloading image...", texture_url.as_str());
    let (image_data, mut descriptor) = match load_data(context, texture_url).await? {
        (img, Some(meta)) => (img, serde_json::from_slice(&meta)?),
        (img, None) => (img, TextureDescriptor::new()),
    };

    log::debug!("[{}] Docompressing image...", texture_url.as_str());
    let mut image = task::spawn_blocking(move || image::load_from_memory(&image_data)).await??;
    log::trace!(
        "[{}] Image:\n  size: {:?}\n  color: {:?}",
        texture_url.as_str(),
        image.dimensions(),
        image.color()
    );

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
    let cooked_dependency = context
        .target_db
        .upload_cooked_binary(
            &asset_base,
            &texture_url.set_extension("tex")?,
            AssetNaming::Hard,
            &cooked_texture,
            Vec::new(),
        )
        .await?;
    context
        .cache_db
        .set_info(texture_url.as_str(), &source_hash, cooked_dependency.url())
        .await?;
    Ok(cooked_dependency)
}
