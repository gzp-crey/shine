use crate::{Context, CookerError, Dependency, SourceId, TargetNaming};
use shine_game::assets::ShaderSource;

pub async fn cook_shader(context: &Context, shader_id: SourceId) -> Result<Dependency, CookerError> {
    let shader_url = shader_id.to_url(&context.source_root)?;

    let ext = shader_url.extension();
    let (source, source_hash) = ShaderSource::load(&context.source_io, &shader_url).await?;

    let cooked = source.cook().await?;
    let cooked_content = bincode::serialize(&cooked)?;

    log::debug!("[{}] Uploading...", shader_url.as_str());
    Ok(context
        .target_db
        .upload_cooked_binary(
            &shader_id,
            &shader_url,
            source_hash,
            TargetNaming::Hard("shader".to_owned(), Some(format!("{}_spv", ext))),
            &cooked_content,
            Vec::new(),
        )
        .await?)
}
