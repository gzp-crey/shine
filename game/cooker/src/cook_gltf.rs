use crate::{Context, CookerError, Dependency, TargetNaming};
use shine_game::assets::{AssetId, GltfSource};

pub async fn cook_gltf(context: &Context, source_id: AssetId) -> Result<Dependency, CookerError> {
    let source_url = source_id.to_url(&context.source_root)?;

    let (source, source_hash) = GltfSource::load(&context.source_io, &source_url).await?;

    let cooked = source.cook().await?;
    let cooked_content = bincode::serialize(&cooked)?;

    log::debug!("[{}] Uploading...", source_url.as_str());
    Ok(context
        .target_db
        .upload_cooked_binary(
            &source_id,
            &source_url,
            source_hash,
            TargetNaming::Soft("model".to_owned(), Some(format!("mdl"))),
            &cooked_content,
            Vec::new(),
        )
        .await?)
}
