use image::{dxt, imageops::FilterType, DynamicImage, GenericImageView, ImageError, ImageOutputFormat};
use shine_game::render::{TextureDescriptor, TextureImage, TextureImageEncoding};
use shine_game::utils::{
    assets,
    url::{Url, UrlError},
};
use std::{error, fmt};
use tokio::task;

#[derive(Debug)]
pub enum Error {
    Asset(assets::AssetError),
    Json(serde_json::Error),
    Bincode(bincode::Error),
    Image(ImageError),
    Runtime(task::JoinError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Asset(ref err) => write!(f, "Asset error: {}", err),
            Error::Json(ref err) => write!(f, "Json error: {}", err),
            Error::Bincode(ref err) => write!(f, "Binary serialize error: {}", err),
            Error::Image(ref err) => write!(f, "Image processing error: {}", err),
            Error::Runtime(ref err) => write!(f, "Runtime error: {}", err),
        }
    }
}

impl error::Error for Error {}

impl From<assets::AssetError> for Error {
    fn from(err: assets::AssetError) -> Error {
        Error::Asset(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::Json(err)
    }
}

impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Error {
        Error::Bincode(err)
    }
}

impl From<UrlError> for Error {
    fn from(err: UrlError) -> Error {
        Error::Asset(assets::AssetError::InvalidUrl(err))
    }
}

impl From<task::JoinError> for Error {
    fn from(err: task::JoinError) -> Error {
        Error::Runtime(err)
    }
}

impl From<ImageError> for Error {
    fn from(err: ImageError) -> Error {
        Error::Image(err)
    }
}

pub async fn load_image(image_url: &Url) -> Result<DynamicImage, Error> {
    log::trace!("[{}] Downloading image...", image_url.as_str());
    let data = assets::download_binary(&image_url).await?;

    log::trace!("[{}] Docompressing image...", image_url.as_str());
    let image = task::spawn_blocking(move || image::load_from_memory(&data)).await??;

    log::trace!(
        "[{}] Image:\n  size: {:?}\n  color: {:?}",
        image_url.as_str(),
        image.dimensions(),
        image.color()
    );
    Ok(image)
}

pub async fn load_descriptor(meta_url: &Url) -> Result<TextureDescriptor, Error> {
    log::trace!("[{}] Downloading descriptor...", meta_url.as_str());
    match assets::download_string(&meta_url).await {
        Ok(data) => Ok(serde_json::from_str(&data)?),
        Err(assets::AssetError::AssetProvider(_)) => {
            log::warn!("[{}] Missing  texture descriptor", meta_url.as_str());
            Ok(TextureDescriptor::new())
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn cook_texture(_source_base: &Url, target_base: &Url, texture_url: &Url) -> Result<String, Error> {
    let mut image = load_image(texture_url).await?;
    let mut descriptor = load_descriptor(&texture_url.set_extension("tex")?).await?;

    if descriptor.size != (0, 0) {
        let (w, h) = descriptor.size;
        log::trace!("[{}] Resizing texture to ({},{})...", texture_url.as_str(), w, h);
        image = task::spawn_blocking(move || image.resize_exact(w, h, FilterType::CatmullRom)).await?;
    } else {
        descriptor.size = image.dimensions();
    }
    log::trace!("[{}] Texture descriptor:\n{:#?}", texture_url.as_str(), descriptor);

    log::trace!("[{}] Compressing texture...", texture_url.as_str());
    let encoding = descriptor.encoding;
    let image = task::spawn_blocking(move || match encoding {
        TextureImageEncoding::Png => {
            let mut image_data = Vec::new();
            image.write_to(&mut image_data, ImageOutputFormat::Png)?;
            Ok::<_, Error>(image_data)
        }
    })
    .await??;

    log::trace!("{}", serde_json::to_string(&descriptor).unwrap());

    log::trace!("[{}] Uploading...", texture_url.as_str());
    let cooked_texture = bincode::serialize(&TextureImage { descriptor, image })?;
    let target_id = assets::upload_cooked_binary(&target_base, "tex", &cooked_texture).await?;
    Ok(target_id)
}
