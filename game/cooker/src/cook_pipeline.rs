use serde_json;
use shine_game::render::PipelineDescriptor;
use shine_game::utils::{assets, url::Url};
use crate::cook_shader;

pub async fn cook(sourc_base: &Url, target_base: &Url, source_id: &str) -> Result<(String, String), String> {
    let source_url = sourc_base
        .join(source_id)
        .map_err(|err| format!("Invalid source url: {:?}", err))?;
    log::trace!("Downloading pipeline descriptor from {}", source_url.as_str());
    let pipeline = assets::download_string(&source_url)
        .await
        .map_err(|err| format!("Failed to get pipeline descriptor: {:?}", err))?;

    let pipeline = serde_json::from_str::<PipelineDescriptor>(&pipeline)
        .map_err(|err| format!("Failed to parse pipeline descriptor: {:?}", err))?;
    log::trace!("Pipeline: {:#?}", pipeline);



    /*    let hash = content_hash::sha256_bytes(shader_source.as_bytes());
    let hash = content_hash::hash_to_path(&hash);
    let target_id = format!("{}.{}_spv", hash, ext);
    let target_url = target_base
        .join(&target_id)
        .map_err(|err| format!("Invalid target url: {:?}", err))?;
    log::trace!("Uploading shader binary as: {}", target_url.as_str());
    assets::upload_binary(&target_url, compiled_artifact.as_binary_u8())
        .await
        .map_err(|err| format!("Failed to upload {}: {:?}", target_url.as_str(), err))?;*/

    Ok(("source_id".to_owned(), "target_id".to_owned()))
}