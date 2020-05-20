use crate::cook_shader;
use shine_game::assets::{AssetError, AssetIO, PipelineDescriptor, Url, UrlError};
use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    Asset(AssetError),
    Json(serde_json::Error),
    Shader(cook_shader::Error),
    Bincode(bincode::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Asset(ref err) => write!(f, "Asset error: {}", err),
            Error::Json(ref err) => write!(f, "Json error: {}", err),
            Error::Shader(ref err) => write!(f, "Shader error: {}", err),
            Error::Bincode(ref err) => write!(f, "Binary serialize error: {}", err),
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

impl From<cook_shader::Error> for Error {
    fn from(err: cook_shader::Error) -> Error {
        Error::Shader(err)
    }
}

pub async fn cook_pipeline(
    io: &AssetIO,
    source_base: &Url,
    target_base: &Url,
    pipeline_url: &Url,
) -> Result<String, Error> {
    log::trace!("[{}] Cooking...", pipeline_url.as_str());

    log::trace!("[{}] Downloading...", pipeline_url.as_str());
    let pipeline = io.download_string(&pipeline_url).await?;

    let mut pipeline = serde_json::from_str::<PipelineDescriptor>(&pipeline)?;
    log::trace!("[{}] Pipeline:\n{:#?}", pipeline_url.as_str(), pipeline);

    let global_layout = pipeline.get_global_uniform_layout()?;
    log::trace!(
        "[{}] Global binding layout:\n{:#?}",
        pipeline_url.as_str(),
        global_layout
    );
    let local_layout = pipeline.get_local_uniform_layout()?;
    log::trace!("[{}] Local binding layout:\n{:#?}", pipeline_url.as_str(), local_layout);

    log::trace!("[{}] Cooking vertex shader...", pipeline_url.as_str());
    let vertex_shader_url = Url::from_base_or_current(&source_base, &pipeline_url, &pipeline.vertex_stage.shader)?;
    let vertex_shader_id = cook_shader::cook_shader(io, source_base, target_base, &vertex_shader_url).await?;
    pipeline.vertex_stage.shader = vertex_shader_id.to_owned();

    log::trace!("[{}] Cooking fragment shader...", pipeline_url.as_str());
    let fragment_shader_url = Url::from_base_or_current(&source_base, &pipeline_url, &pipeline.fragment_stage.shader)?;
    let fragment_shader_id = cook_shader::cook_shader(io, source_base, target_base, &fragment_shader_url).await?;
    pipeline.fragment_stage.shader = fragment_shader_id.to_owned();

    log::trace!("[{}] Uploading...", pipeline_url.as_str());
    let cooked_pipeline = bincode::serialize(&pipeline)?;
    let target_id = io.upload_cooked_binary(&target_base, "pl", &cooked_pipeline).await?;

    Ok(target_id)
}
