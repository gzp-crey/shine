use crate::{cook_pipeline, cook_texture, AssetNaming, Context, CookingError, Dependency};
use shine_game::assets::{AssetId, FrameGraphDescriptor, PassMethod, Url};

async fn find_frame_graph_etag(context: &Context, frame_graph_url: &Url) -> Result<String, CookingError> {
    Ok(context.source_io.download_etag(&frame_graph_url).await?)
}

pub async fn get_frame_graph_etag(
    context: &Context,
    asset_base: &Url,
    frame_graph_id: &AssetId,
) -> Result<String, CookingError> {
    let frame_graph_url = frame_graph_id.to_url(asset_base)?;
    find_frame_graph_etag(context, &frame_graph_url).await
}

pub async fn cook_frame_graph(
    context: &Context,
    asset_base: &Url,
    frame_graph_id: &AssetId,
) -> Result<Dependency, CookingError> {
    let frame_graph_url = frame_graph_id.to_url(asset_base)?;

    log::info!("[{}] Cooking...", frame_graph_url.as_str());
    let source_hash = find_frame_graph_etag(context, &frame_graph_url).await?;

    log::debug!("[{}] Downloading...", frame_graph_url.as_str());
    let data = context.source_io.download_binary(&frame_graph_url).await?;
    let mut frame_graph = serde_json::from_slice::<FrameGraphDescriptor>(&data)?;
    log::trace!("[{}] Frame graph:\n{:#?}", frame_graph_url.as_str(), frame_graph);

    let mut dependnecies = Vec::new();
    let frame_graph_base = frame_graph_url.to_folder()?;

    log::debug!("[{}] Cooking frame graph content...", frame_graph_url.as_str());
    for ref mut pass in frame_graph.passes.values_mut() {
        match pass.method {
            PassMethod::FullScreenQuad(ref mut pipeline) => {
                log::debug!("[{}] Cooking FullScreenQuad pipeline...", frame_graph_url.as_str());
                let pipeline_id = AssetId::new(&pipeline)?.to_absolute_id(asset_base, &frame_graph_base)?;
                let pipeline_dependency = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_id).await?;
                *pipeline = pipeline_dependency.url().as_str().to_owned();
                dependnecies.push(pipeline_dependency);
            }
        }
    }

    log::trace!("[{}] Cooked frame graph:\n{:#?}", frame_graph_url.as_str(), frame_graph);

    log::debug!("[{}] Uploading...", frame_graph_url.as_str());
    let cooked_frame_graph = bincode::serialize(&frame_graph)?;
    Ok(context
        .target_db
        .upload_cooked_binary(
            frame_graph_id.clone(),
            frame_graph_url,
            AssetNaming::SoftScheme("framegraph".to_owned()),
            &cooked_frame_graph,
            source_hash,
            dependnecies,
        )
        .await?)
}
