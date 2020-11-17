use crate::{Context, CookerError, Dependency, SourceId, TargetNaming};
use shine_game::assets::TextureSource;

pub async fn cook_texture(context: &Context, texture_id: SourceId) -> Result<Dependency, CookerError> {
    let texture_url = texture_id.to_url(&context.source_root)?;

    let (source, source_hash) = TextureSource::load(&context.source_io, &texture_url).await?;

    let cooked = source.cook().await?;
    let cooked_content = bincode::serialize(&cooked)?;

    log::debug!("[{}] Uploading...", texture_url.as_str());
    Ok(context
        .target_db
        .upload_cooked_binary(
            &texture_id,
            &texture_url,
            source_hash,
            TargetNaming::Hard("texture".to_owned(), Some("tx".to_owned())),
            &cooked_content,
            Vec::new(),
        )
        .await?)
}
