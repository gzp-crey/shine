use crate::content_hash::upload_cooked_binary;
use shaderc;
use shine_game::utils::{assets, url::Url};

pub async fn cook_shader(_source_base: &Url, target_base: &Url, shader_url: &Url) -> Result<String, String> {
    log::trace!("Downloading shader source from [{}]", shader_url.as_str());
    let shader_source = assets::download_string(&shader_url)
        .await
        .map_err(|err| format!("Failed to get source content [{}]: {:?}", shader_url.as_str(), err))?;

    let ext = shader_url.extension();
    let ty = match ext {
        "vs" => shaderc::ShaderKind::Vertex,
        "fs" => shaderc::ShaderKind::Fragment,
        "cs" => shaderc::ShaderKind::Compute,
        _ => return Err(format!("Unknown shader type [{}]: [{}]", shader_url.as_str(), ext)),
    };

    log::trace!("Compiling {:?} shader: {}", ty, shader_source);
    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();
    let compiled_artifact = compiler
        .compile_into_spirv(&shader_source, ty, shader_url.as_str(), "main", Some(&options))
        .map_err(|err| format!("Shader compilation [{}] failed: {:?}", shader_url.as_str(), err))?;

    let target_id =
        upload_cooked_binary(&target_base, &format!("{}_spv", ext), compiled_artifact.as_binary_u8()).await?;
    Ok(target_id)
}
