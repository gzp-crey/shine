use crate::{Context, CookerError, Dependency, TargetNaming, cook_shader};
use shine_game::assets::{AssetId, PipelineSource};

pub async fn cook_pipeline(context: &Context, source_id: AssetId) -> Result<Dependency, CookerError> {
    let source_url = source_id.to_url(&context.source_root)?;

    let (mut source, source_hash) = PipelineSource::load(&context.source_io, &source_url).await?;

    let mut dependencies = Vec::new();
    {
        let vs = &mut source.descriptor.vertex_stage;
        let vs_id = source_id.new_relative(&vs.shader)?;
        let dep = cook_shader::cook_shader(context, vs_id).await?;
        vs.shader = dep.cooked_id();
        dependencies.push(dep);
    }

    {
        let fs = &mut source.descriptor.fragment_stage;
        let fs_id = source_id.new_relative(&fs.shader)?;
        let dep = cook_shader::cook_shader(context, fs_id).await?;
        fs.shader = dep.cooked_id();
        dependencies.push(dep);
    }

    let cooked = source.cook().await?;
    let cooked_content = bincode::serialize(&cooked)?;

    log::debug!("[{}] Uploading...", source_url.as_str());
    Ok(context
        .target_db
        .upload_cooked_binary(
            &source_id,
            &source_url,
            source_hash,
            TargetNaming::Soft("pipeline".to_owned(), None),
            &cooked_content,
            dependencies,
        )
        .await?)
}
