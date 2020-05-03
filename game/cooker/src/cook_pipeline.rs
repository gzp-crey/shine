use crate::content_hash;
use crate::cook_shader::cook_shader;
use serde_json;
use shine_game::render::PipelineDescriptor;
use shine_game::utils::{assets, url::Url};

pub async fn cook_pipeline(source_base: &Url, target_base: &Url, pipeline_url: &Url) -> Result<String, String> {
    log::trace!("Downloading pipeline descriptor from [{}]", pipeline_url.as_str());
    let pipeline = assets::download_string(&pipeline_url).await.map_err(|err| {
        format!(
            "Failed to get pipeline descriptor [{}]: {:?}",
            pipeline_url.as_str(),
            err
        )
    })?;

    let mut pipeline = serde_json::from_str::<PipelineDescriptor>(&pipeline).map_err(|err| {
        format!(
            "Failed to parse pipeline descriptor [{}]: {:?}",
            pipeline_url.as_str(),
            err
        )
    })?;
    log::trace!("Pipeline: {:#?}", pipeline);

    let vertex_shader_url = Url::from_base_or_current(&source_base, &pipeline_url, &pipeline.vertex_stage.shader)
        .map_err(|err| {
            format!(
                "Failed to get vertex shader url [{}]/[{}]: {:?}",
                pipeline_url.as_str(),
                pipeline.vertex_stage.shader,
                err
            )
        })?;
    let vertex_shader_id = cook_shader(source_base, target_base, &vertex_shader_url)
        .await
        .map_err(|err| {
            format!(
                "Failed to cook pipeline [{}] due to vertex shader [{}]: {:?}",
                pipeline_url.as_str(),
                pipeline.vertex_stage.shader,
                err
            )
        })?;
    pipeline.vertex_stage.shader = vertex_shader_id.to_owned();

    let fragment_shader_url = Url::from_base_or_current(&source_base, &pipeline_url, &pipeline.fragment_stage.shader)
        .map_err(|err| {
        format!(
            "Failed to get fragment shader url [{}]/[{}]: {:?}",
            pipeline_url.as_str(),
            pipeline.fragment_stage.shader,
            err
        )
    })?;
    let fragment_shader_id = cook_shader(source_base, target_base, &fragment_shader_url)
        .await
        .map_err(|err| {
            format!(
                "Failed to cook pipeline [{}] due to fragment shader [{}] dependecy: {:?}",
                pipeline_url.as_str(),
                pipeline.vertex_stage.shader,
                err
            )
        })?;
    pipeline.fragment_stage.shader = fragment_shader_id.to_owned();

    let cooked_pipeline = serde_json::to_string(&pipeline)
        .map_err(|err| format!("Failed to serialize pipeline [{}]: {:?}", pipeline_url.as_str(), err))?;

    let hash = content_hash::sha256_bytes(cooked_pipeline.as_bytes());
    let hash = content_hash::hash_to_path(&hash);
    let target_id = format!("{}.pl", hash);
    let target_url = target_base
        .join(&target_id)
        .map_err(|err| format!("Invalid target url: {:?}", err))?;
    log::trace!("Uploading pipeline binary [{}]", target_url.as_str());
    assets::upload_string(&target_url, &cooked_pipeline)
        .await
        .map_err(|err| format!("Failed to upload [{}]: {:?}", target_url.as_str(), err))?;

    Ok(target_id)
}
