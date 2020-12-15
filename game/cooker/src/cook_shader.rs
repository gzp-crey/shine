use crate::{Context, CookerError, Dependency, TargetNaming};
use shine_game::assets::{AssetId, ShaderSource};

pub async fn cook_shader(context: &Context, source_id: AssetId) -> Result<Url, CookerError> {
    let source_url = source_id.to_url(&context.source_root)?;

    let ext = source_url.extension();
    let (source, source_hash) = ShaderSource::load(&context.source_io, &source_url).await?;

    let cooked = source.cook().await?;
    let cooked_content = bincode::serialize(&cooked)?;

    log::debug!("[{}] Uploading...", source_url.as_str());
    Ok(context
        .target_db
        .upload_cooked_binary(
            &source_id,
            &source_url,
            source_hash,
            TargetNaming::Hard("shader".to_owned(), Some(format!("{}_spv", ext))),
            &cooked_content,
        )
        .await?)
}
