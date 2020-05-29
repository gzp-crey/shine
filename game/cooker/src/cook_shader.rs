use crate::{Context, CookingError};
use shine_game::assets::{AssetError, AssetNaming, Url};

impl From<shaderc::Error> for CookingError {
    fn from(err: shaderc::Error) -> CookingError {
        AssetError::Content(format!("Shader compilation failed: {}", err)).into()
    }
}

pub async fn cook_shader(
    context: &Context,
    asset_base: &Url,
    shader_url: &Url,
) -> Result<Url, CookingError> {
    log::debug!("[{}] Cooking...", shader_url.as_str());

    log::debug!("[{}] Downloading...", shader_url.as_str());
    let shader_source = context.source_io.download_string(&shader_url).await?;
    let ext = shader_url.extension();
    let ty = match ext {
        "vs" => Ok(shaderc::ShaderKind::Vertex),
        "fs" => Ok(shaderc::ShaderKind::Fragment),
        "cs" => Ok(shaderc::ShaderKind::Compute),
        _ => Err(AssetError::UnsupportedFormat(ext.to_owned())),
    }?;

    log::debug!("[{}] Compiling...", shader_url.as_str());
    log::trace!("[{}] Source:\n{}", shader_url.as_str(), shader_source);
    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();
    let compiled_artifact =
        compiler.compile_into_spirv(&shader_source, ty, shader_url.as_str(), "main", Some(&options))?;

    log::debug!("[{}] Uploading...", shader_url.as_str());
    Ok(context
        .target_io
        .upload_cooked_binary(
            &asset_base,
            &shader_url.set_extension(&format!("{}_spv", ext))?,
            AssetNaming::Hash,
            compiled_artifact.as_binary_u8(),
        )
        .await?)
}
