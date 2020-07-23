use crate::{cook_shader, AssetNaming, Context, CookingError, Dependency};
use shine_game::assets::{AssetId, PipelineDescriptor, Url};
use shine_game::render::MAX_UNIFORM_GROUP_COUNT;

async fn find_pipeline_etag(context: &Context, pipeline_url: &Url) -> Result<String, CookingError> {
    Ok(context.source_io.download_etag(&pipeline_url).await?)
}

pub async fn get_pipeline_etag(
    context: &Context,
    asset_base: &Url,
    pipeline_id: &AssetId,
) -> Result<String, CookingError> {
    let pipeline_url = pipeline_id.to_url(&asset_base)?;
    find_pipeline_etag(context, &pipeline_url).await
}

pub async fn cook_pipeline(
    context: &Context,
    asset_base: &Url,
    pipeline_id: &AssetId,
) -> Result<Dependency, CookingError> {
    let pipeline_url = pipeline_id.to_url(&asset_base)?;

    log::debug!("[{}] Cooking...", pipeline_url.as_str());
    let source_hash = find_pipeline_etag(context, &pipeline_url).await?;

    log::debug!("[{}] Downloading...", pipeline_url.as_str());
    let pipeline = context.source_io.download_binary(&pipeline_url).await?;
    let mut pipeline = serde_json::from_slice::<PipelineDescriptor>(&pipeline)?;
    log::trace!("[{}] Pipeline:\n{:#?}", pipeline_url.as_str(), pipeline);

    let mut dependencies = Vec::new();

    for i in 0..MAX_UNIFORM_GROUP_COUNT {
        let layout = pipeline.get_uniform_layout(i)?;
        log::trace!(
            "[{}] Uniform group({}) layout:\n{:#?}",
            pipeline_url.as_str(),
            i,
            layout
        );
    }

    let pipeline_base = pipeline_url.to_folder()?;

    log::debug!("[{}] Cooking vertex shader...", pipeline_url.as_str());
    let vertex_shader_id = AssetId::new(&pipeline.vertex_stage.shader)?.to_absolute_id(asset_base, &pipeline_base)?;
    let vertex_shader_dependency = cook_shader::cook_shader(context, asset_base, &vertex_shader_id).await?;
    pipeline.vertex_stage.shader = vertex_shader_dependency.url().as_str().to_owned();
    dependencies.push(vertex_shader_dependency);

    log::debug!("[{}] Cooking fragment shader...", pipeline_url.as_str());
    let fragment_shader_id =
        AssetId::new(&pipeline.fragment_stage.shader)?.to_absolute_id(asset_base, &pipeline_base)?;
    let fragment_shader_dependency = cook_shader::cook_shader(context, asset_base, &fragment_shader_id).await?;
    pipeline.fragment_stage.shader = fragment_shader_dependency.url().as_str().to_owned();
    dependencies.push(fragment_shader_dependency);

    log::debug!("[{}] Uploading...", pipeline_url.as_str());
    let cooked_pipeline = bincode::serialize(&pipeline)?;
    Ok(context
        .target_db
        .upload_cooked_binary(
            pipeline_id.clone(),
            pipeline_url,
            AssetNaming::Hard("pipeline".to_owned()),
            &cooked_pipeline,
            source_hash,
            dependencies,
        )
        .await?)
}
