use crate::{AssetNaming, Context, CookingError, TargetDependency};
use shine_game::assets::{AssetError, Url};

impl From<shaderc::Error> for CookingError {
    fn from(err: shaderc::Error) -> CookingError {
        AssetError::Content(format!("Shader compilation failed: {}", err)).into()
    }
}

pub async fn get_shader_etag(context: &Context, shader_url: &Url) -> Result<String, CookingError> {
    Ok(context.source_io.download_etag(&shader_url).await?)
}

pub async fn cook_shader(
    context: &Context,
    asset_base: &Url,
    shader_id: &AssetId,
) -> Result<TargetDependency, CookingError> {
    let shader_url = shader_id.to_url(asset_base)?;
    
    log::debug!("[{}] Cooking...", shader_url.as_str());
    let source_hash = get_shader_etag(context, &shader_url).await?;

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
    let cooked_dependency = context
        .target_db
        .upload_cooked_binary(
            shader_id,
            &shader_url.set_extension(&format!("{}_spv", ext))?,
            AssetNaming::Hard,
            compiled_artifact.as_binary_u8(),
            Vec::new(),
        )
        .await?;
    context
        .cache_db
        .set_info(shader_url.as_str(), &source_hash, shader_id.as_str())
        .await?;
    Ok(cooked_dependency)
}
