use crate::content_hash::upload_cooked_binary;
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
    log::trace!("Pipeline [{}]: {:#?}", pipeline_url.as_str(), pipeline);

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

    let cooked_pipeline = bincode::serialize(&pipeline)
        .map_err(|err| format!("Failed to serialize pipeline [{}]: {:?}", pipeline_url.as_str(), err))?;

    let target_id = upload_cooked_binary(&target_base, "pl", &cooked_pipeline).await?;
    Ok(target_id)
}
