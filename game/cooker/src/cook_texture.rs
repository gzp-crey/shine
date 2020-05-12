use crate::content_hash::upload_cooked_binary;
use image::{dxt, imageops::FilterType, DynamicImage, GenericImageView, ImageOutputFormat};
use shine_game::render::{TextureDescriptor, TextureImage, TextureImageEncoding};
use shine_game::utils::{assets, url::Url};
use std::io::Write;
use tokio::task;

pub async fn load_image(image_url: &Url) -> Result<DynamicImage, String> {
    log::trace!("Downloading image [{}]", image_url.as_str());
    let data = assets::download_binary(&image_url)
        .await
        .map_err(|err| format!("Failed to get texture image [{}]: {:?}", image_url.as_str(), err))?;

    log::trace!("Docompressing image [{}]", image_url.as_str());
    let image = task::spawn_blocking(move || image::load_from_memory(&data))
        .await
        .map_err(|err| format!("Failed spawn decode image [{}] task: {:?}", image_url.as_str(), err))?
        .map_err(|err| format!("Failed to decode image [{}]: {:?}", image_url.as_str(), err))?;

    log::trace!(
        "Image loaded from [{}]: {:?}({:?})",
        image_url.as_str(),
        image.dimensions(),
        image.color()
    );
    Ok(image)
}

pub async fn load_descriptor(meta_url: &Url) -> Result<TextureDescriptor, String> {
    log::trace!("Downloading image [{}]", meta_url.as_str());
    match assets::download_string(&meta_url).await {
        Ok(data) => serde_json::from_str(&data)
            .map_err(|err| format!("Failed to load texture descriptor [{}]: {:?}", meta_url.as_str(), err)),
        Err(assets::AssetError::AssetProvider(_)) => {
            log::warn!("No texture descriptor: [{}]", meta_url.as_str());
            Ok(TextureDescriptor::new())
        }
        Err(err) => Err(format!(
            "Failed to get texture descriptor [{}]: {:?}",
            meta_url.as_str(),
            err
        )),
    }
}

pub async fn cook_texture(_source_base: &Url, target_base: &Url, texture_url: &Url) -> Result<String, String> {
    let mut image = load_image(texture_url).await?;
    let mut descriptor = load_descriptor(&texture_url.set_extension("tex").unwrap()).await?;

    if descriptor.size != (0, 0) {
        let size = descriptor.size;
        image = task::spawn_blocking(move || image.resize_exact(size.0, size.1, FilterType::CatmullRom))
            .await
            .map_err(|err| {
                format!(
                    "Failed spawn resize image task for [{}]: {:?}",
                    texture_url.as_str(),
                    err
                )
            })?;
    } else {
        descriptor.size = image.dimensions();
    }
    log::trace!("Texture [{}]: ({:#?})", texture_url.as_str(), descriptor);

    log::trace!("Compressing texture [{}]...", texture_url.as_str());
    let encoding = descriptor.encoding;
    let image = task::spawn_blocking(move || {
        let mut image_data = Vec::new();
        match encoding {
            TextureImageEncoding::Png => image.write_to(&mut image_data, ImageOutputFormat::Png).unwrap(),
            //.map_err(|err| format!("{:?}", err))?,
        };
        image_data
    })
    .await
    .map_err(|err| {
        format!(
            "Failed spawn compress image task for [{}]: {:?}",
            texture_url.as_str(),
            err
        )
    })?;
    //.map_err(|err| format!("Failed to compress image [{}]: {:?}", texture_url.as_str(), err))?;

    let texture_image = TextureImage { descriptor, image };
    let cooked_texture =
        bincode::serialize(&texture_image).map_err(|err| format!("Failed to compose texture image: {:?}", err))?;

    let target_id = upload_cooked_binary(&target_base, "tex", &cooked_texture).await?;
    Ok(target_id)
}
