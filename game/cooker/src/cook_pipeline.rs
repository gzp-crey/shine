use crate::content_hash;
use crate::cook_shader;
use serde_json;
use shine_game::render::PipelineDescriptor;
use shine_game::utils::{assets, url::Url};

pub async fn cook(source_base: &Url, target_base: &Url, source_id: &str) -> Result<String, String> {
    let source_url = source_base
        .join(source_id)
        .map_err(|err| format!("Invalid source url: {:?}", err))?;
    log::trace!("Downloading pipeline descriptor from {}", source_url.as_str());
    let pipeline = assets::download_string(&source_url)
        .await
        .map_err(|err| format!("Failed to get pipeline descriptor: {:?}", err))?;

    let mut pipeline = serde_json::from_str::<PipelineDescriptor>(&pipeline)
        .map_err(|err| format!("Failed to parse pipeline descriptor: {:?}", err))?;
    log::trace!("Pipeline: {:#?}", pipeline);

    let vertex_shader_id = cook_shader::cook(source_base, target_base, &pipeline.vertex_stage.shader)
        .await
        .map_err(|err| {
            format!(
                "Failed to cook pipeline due to vertex shader ({}) dependecy: {:?}",
                pipeline.vertex_stage.shader, err
            )
        })?;
    pipeline.vertex_stage.shader = vertex_shader_id.to_owned();

    let fragment_shader_id = cook_shader::cook(source_base, target_base, &pipeline.fragment_stage.shader)
        .await
        .map_err(|err| {
            format!(
                "Failed to cook pipeline due to fragment shader ({}) dependecy: {:?}",
                pipeline.vertex_stage.shader, err
            )
        })?;
    pipeline.fragment_stage.shader = fragment_shader_id.to_owned();

    let cooked_pipeline =
        serde_json::to_string(&pipeline).map_err(|err| format!("Failed to erialize pipeline: {:?}", err))?;

    let hash = content_hash::sha256_bytes(cooked_pipeline.as_bytes());
    let hash = content_hash::hash_to_path(&hash);
    let target_id = format!("{}.pl", hash);
    let target_url = target_base
        .join(&target_id)
        .map_err(|err| format!("Invalid target url: {:?}", err))?;
    log::trace!("Uploading pipeline binary as: {}", target_url.as_str());
    assets::upload_string(&target_url, &cooked_pipeline)
        .await
        .map_err(|err| format!("Failed to upload {}: {:?}", target_url.as_str(), err))?;

    Ok("target_id".to_owned())
}
