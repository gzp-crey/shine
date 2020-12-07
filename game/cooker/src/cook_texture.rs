use crate::{Context, CookerError, Dependency, SourceId, TargetNaming};
use shine_game::assets::TextureSource;

pub async fn cook_texture(context: &Context, source_id: SourceId) -> Result<Dependency, CookerError> {
    let source_url = source_id.to_url(&context.source_root)?;

    let (source, source_hash) = TextureSource::load(&context.source_io, &source_url).await?;

    let cooked = source.cook().await?;
    let cooked_content = bincode::serialize(&cooked)?;

    log::debug!("[{}] Uploading...", source_url.as_str());
    Ok(context
        .target_db
        .upload_cooked_binary(
            &source_id,
            &source_url,
            source_hash,
            TargetNaming::Hard("texture".to_owned(), Some("tx".to_owned())),
            &cooked_content,
            Vec::new(),
        )
        .await?)
}
