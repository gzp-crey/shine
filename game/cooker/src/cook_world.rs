use crate::{cook_pipeline, cook_texture};
use shine_game::assets::{AssetError, AssetIO, Url, UrlError};
use shine_game::world::World;
use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    Asset(AssetError),
    Json(serde_json::Error),
    Bincode(bincode::Error),
    Resource(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Asset(ref err) => write!(f, "Asset error: {}", err),
            Error::Json(ref err) => write!(f, "Json error: {}", err),
            Error::Bincode(ref err) => write!(f, "Binary serialize error: {}", err),
            Error::Resource(ref err) => write!(f, "Referenced resource error: {}", err),
        }
    }
}

impl error::Error for Error {}

impl From<AssetError> for Error {
    fn from(err: AssetError) -> Error {
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
        Error::Asset(AssetError::InvalidUrl(err))
    }
}

impl From<cook_pipeline::Error> for Error {
    fn from(err: cook_pipeline::Error) -> Error {
        Error::Resource(format!("Referenced pipeline error: {}", err))
    }
}

impl From<cook_texture::Error> for Error {
    fn from(err: cook_texture::Error) -> Error {
        Error::Resource(format!("Referenced texture error: {}", err))
    }
}

pub async fn cook_world(
    io: &AssetIO,
    source_base: &Url,
    target_base: &Url,
    world_url: &Url,
    target_url: &Url,
) -> Result<String, Error> {
    log::info!("[{}] Cooking...", world_url.as_str());

    log::debug!("[{}] Downloading...", world_url.as_str());
    let data = io.download_binary(&world_url).await?;
    let mut world = serde_json::from_slice::<World>(&data)?;
    log::trace!("[{}] World:\n{:#?}", world_url.as_str(), world);

    log::debug!("[{}] Cooking world content...", world_url.as_str());
    match world {
        World::Test1(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(source_base, world_url, &test.pipeline)?;
            test.pipeline = cook_pipeline::cook_pipeline(io, source_base, target_base, &pipeline_url).await?;
        }
        World::Test2(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(source_base, world_url, &test.pipeline)?;
            test.pipeline = cook_pipeline::cook_pipeline(io, source_base, target_base, &pipeline_url).await?;
        }
        World::Test3(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(source_base, world_url, &test.pipeline)?;
            test.pipeline = cook_pipeline::cook_pipeline(io, source_base, target_base, &pipeline_url).await?;
            let texture_url = Url::from_base_or_current(source_base, world_url, &test.texture)?;
            test.texture = cook_texture::cook_texture(io, source_base, target_base, &texture_url).await?;
        }
    }
    log::trace!("[{}] Cooked world:\n{:#?}", world_url.as_str(), world);

    log::debug!("[{}] Uploading...", world_url.as_str());
    let cooked_world = bincode::serialize(&world)?;
    io.upload_binary(&target_url, &cooked_world).await?;

    Ok(target_url.as_str().to_owned())
}
