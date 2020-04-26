use crate::content_hash;
use shaderc;
use shine_game::utils::{assets, url::Url};

pub async fn cook(sourc_base: &Url, target_base: &Url, source_id: &str) -> Result<String, String> {
    let source_url = sourc_base
        .join(source_id)
        .map_err(|err| format!("Invalid source url: {:?}", err))?;
    log::trace!("Downloading shader source from {}", source_url.as_str());
    let shader_source = assets::download_string(&source_url)
        .await
        .map_err(|err| format!("Failed to get source content [{}]: {:?}", source_id, err))?;

    let ext = source_url.extension();
    let ty = match ext {
        "vs" => shaderc::ShaderKind::Vertex,
        "fs" => shaderc::ShaderKind::Fragment,
        "cs" => shaderc::ShaderKind::Compute,
        _ => return Err(format!("Unknown shader type: {}", ext)),
    };
    log::trace!("Compiling {:?} shader", ty);
    let mut compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();
    let compiled_artifact = compiler
        .compile_into_spirv(&shader_source, ty, source_url.as_str(), "main", Some(&options))
        .map_err(|err| format!("Shader compilation failed: {:?}", err))?;

    let hash = content_hash::sha256_bytes(compiled_artifact.as_binary_u8());
    let hash = content_hash::hash_to_path(&hash);
    let target_id = format!("{}.{}_spv", hash, ext);
    let target_url = target_base
        .join(&target_id)
        .map_err(|err| format!("Invalid target url: {:?}", err))?;
    log::trace!("Uploading shader binary as: {}", target_url.as_str());
    assets::upload_binary(&target_url, compiled_artifact.as_binary_u8())
        .await
        .map_err(|err| format!("Failed to upload {}: {:?}", target_url.as_str(), err))?;

    Ok(target_id)
}
