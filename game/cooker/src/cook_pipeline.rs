use crate::{cook_shader, Context, CookingError};
use shine_game::assets::{PipelineDescriptor, Url, UNIFORM_GROUP_COUNT};

pub async fn cook_pipeline(
    context: &Context,
    source_base: &Url,
    target_base: &Url,
    pipeline_url: &Url,
) -> Result<String, CookingError> {
    log::debug!("[{}] Cooking...", pipeline_url.as_str());

    log::debug!("[{}] Downloading...", pipeline_url.as_str());
    let pipeline = context.assetio.download_string(&pipeline_url).await?;
    let mut pipeline = serde_json::from_str::<PipelineDescriptor>(&pipeline)?;
    log::trace!("[{}] Pipeline:\n{:#?}", pipeline_url.as_str(), pipeline);

    for i in 0..UNIFORM_GROUP_COUNT {
        let layout = pipeline.get_uniform_layout(i)?;
        log::trace!(
            "[{}] Uniform group({}) layout:\n{:#?}",
            pipeline_url.as_str(),
            i,
            layout
        );
    }

    log::debug!("[{}] Cooking vertex shader...", pipeline_url.as_str());
    let vertex_shader_url = Url::from_base_or_current(source_base, pipeline_url, &pipeline.vertex_stage.shader)?;
    let vertex_shader_id = cook_shader::cook_shader(context, source_base, target_base, &vertex_shader_url).await?;
    pipeline.vertex_stage.shader = vertex_shader_id.to_owned();

    log::debug!("[{}] Cooking fragment shader...", pipeline_url.as_str());
    let fragment_shader_url = Url::from_base_or_current(source_base, pipeline_url, &pipeline.fragment_stage.shader)?;
    let fragment_shader_id = cook_shader::cook_shader(context, source_base, target_base, &fragment_shader_url).await?;
    pipeline.fragment_stage.shader = fragment_shader_id.to_owned();

    log::debug!("[{}] Uploading...", pipeline_url.as_str());
    let cooked_pipeline = bincode::serialize(&pipeline)?;
    let target_id = context
        .assetio
        .upload_cooked_binary(&target_base, "pl", &cooked_pipeline)
        .await?;

    Ok(target_id)
}
