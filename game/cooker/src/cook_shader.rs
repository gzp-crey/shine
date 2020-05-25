use shine_game::assets::{AssetError, AssetIO, Url};
use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    Asset(AssetError),
    Json(serde_json::Error),
    UnknownShader(String),
    Compile(shaderc::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Asset(ref err) => write!(f, "Asset error: {}", err),
            Error::Json(ref err) => write!(f, "Json error: {}", err),
            Error::UnknownShader(ref ext) => write!(f, "Unknown shader type: {}", ext),
            Error::Compile(ref err) => write!(f, "Shader compilation error: {}", err),
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

impl From<shaderc::Error> for Error {
    fn from(err: shaderc::Error) -> Error {
        Error::Compile(err)
    }
}

pub async fn cook_shader(
    io: &AssetIO,
    _source_base: &Url,
    target_base: &Url,
    shader_url: &Url,
) -> Result<String, Error> {
    log::debug!("[{}] Cooking...", shader_url.as_str());

    log::debug!("[{}] Downloading...", shader_url.as_str());
    let shader_source = io.download_string(&shader_url).await?;

    let ext = shader_url.extension();
    let ty = match ext {
        "vs" => shaderc::ShaderKind::Vertex,
        "fs" => shaderc::ShaderKind::Fragment,
        "cs" => shaderc::ShaderKind::Compute,
        _ => return Err(Error::UnknownShader(ext.to_owned())),
    };

    log::debug!("[{}] Compiling...", shader_url.as_str());
    log::trace!("[{}] Source:\n{}", shader_url.as_str(), shader_source);
    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();
    let compiled_artifact =
        compiler.compile_into_spirv(&shader_source, ty, shader_url.as_str(), "main", Some(&options))?;

    log::debug!("[{}] Uploading...", shader_url.as_str());
    let target_id = io
        .upload_cooked_binary(&target_base, &format!("{}_spv", ext), compiled_artifact.as_binary_u8())
        .await?;
    Ok(target_id)
}
