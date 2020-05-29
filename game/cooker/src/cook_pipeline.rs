use crate::{cook_shader, Context, CookingError};
use shine_game::assets::{AssetNaming, PipelineDescriptor, Url, UNIFORM_GROUP_COUNT};

pub async fn cook_pipeline(
    context: &Context,
    asset_base: &Url,
    pipeline_url: &Url,
) -> Result<Url, CookingError> {
    log::debug!("[{}] Cooking...", pipeline_url.as_str());

    log::debug!("[{}] Downloading...", pipeline_url.as_str());
    let pipeline = context.source_io.download_string(&pipeline_url).await?;
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
    let vertex_shader_url = Url::from_base_or_current(asset_base, pipeline_url, &pipeline.vertex_stage.shader)?;
    let vertex_shader_id = cook_shader::cook_shader(context, asset_base, &vertex_shader_url).await?;
    pipeline.vertex_stage.shader = vertex_shader_id.as_str().to_owned();

    log::debug!("[{}] Cooking fragment shader...", pipeline_url.as_str());
    let fragment_shader_url = Url::from_base_or_current(asset_base, pipeline_url, &pipeline.fragment_stage.shader)?;
    let fragment_shader_id = cook_shader::cook_shader(context, asset_base, &fragment_shader_url).await?;
    pipeline.fragment_stage.shader = fragment_shader_id.as_str().to_owned();

    log::debug!("[{}] Uploading...", pipeline_url.as_str());
    let cooked_pipeline = bincode::serialize(&pipeline)?;
    Ok(context
        .target_io
        .upload_cooked_binary(
            &asset_base,
            &pipeline_url,
            AssetNaming::VirtualScheme("pipeline".to_owned()),
            &cooked_pipeline,
        )
        .await?)
}
