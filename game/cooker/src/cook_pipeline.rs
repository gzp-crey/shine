use crate::{cook_shader, AssetNaming, Context, CookingError, TargetDependency};
use shine_game::assets::{PipelineDescriptor, Url, AssetId, UNIFORM_GROUP_COUNT};

pub async fn get_pipeline_etag(context: &Context, pipeline_url: &Url) -> Result<String, CookingError> {
    Ok(context.source_io.download_etag(&pipeline_url).await?)
}

pub async fn cook_pipeline(
    context: &Context,
    asset_base: &Url,
    pipeline_id: &AssetId,
) -> Result<TargetDependency, CookingError> {
    let pipeline_url = pipeline_id.to_url(&asset_base)?;

    log::debug!("[{}] Cooking...", pipeline_url.as_str());
    let source_hash = get_pipeline_etag(context, &pipeline_url).await?;

    log::debug!("[{}] Downloading...", pipeline_url.as_str());
    let pipeline = context.source_io.download_binary(&pipeline_url).await?;
    let mut pipeline = serde_json::from_slice::<PipelineDescriptor>(&pipeline)?;
    log::trace!("[{}] Pipeline:\n{:#?}", pipeline_url.as_str(), pipeline);

    let mut dependencies = Vec::new();

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
    let vertex_shader_id = AssetId::new(&pipeline.vertex_stage.shader)?;
    let vertex_shader_base = vertex_shader_id.get_base(asset_base, &pipeline_url.to_folder()?);
    let vertex_shader_dependency = cook_shader::cook_shader(context, vertex_shader_base, &vertex_shader_id).await?;
    pipeline.vertex_stage.shader = vertex_shader_dependency.url().to_owned();
    dependencies.push(vertex_shader_dependency);

    log::debug!("[{}] Cooking fragment shader...", pipeline_url.as_str());
    let fragment_shader_id = AssetId::new(&pipeline.fragment_stage.shader)?;
    let fragment_shader_base = fragment_shader_id.get_base(asset_base, &pipeline_url.to_folder()?);
    let fragment_shader_dependency = cook_shader::cook_shader(context, fragment_shader_base, &fragment_shader_id).await?;
    pipeline.fragment_stage.shader = fragment_shader_dependency.url().to_owned();
    dependencies.push(fragment_shader_dependency);

    log::debug!("[{}] Uploading...", pipeline_url.as_str());
    let cooked_pipeline = bincode::serialize(&pipeline)?;
    let cooked_dependency = context
        .target_db
        .upload_cooked_binary(
            pipeline_id,
            &pipeline_url,
            AssetNaming::SoftScheme("pipeline".to_owned()),
            &cooked_pipeline,
            dependencies,
        )
        .await?;
    context
        .cache_db
        .set_info(pipeline_url.as_str(), &source_hash, cooked_dependency.url())
        .await?;
    Ok(cooked_dependency)
}
