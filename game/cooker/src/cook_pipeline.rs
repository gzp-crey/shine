use crate::{cook_shader, Context, CookerError, Dependency, TargetNaming};
use shine_game::assets::{AssetId, CookingError, PipelineSource, ShaderType};
use std::sync::{Arc, Mutex};

pub async fn cook_pipeline(context: &Context, source_id: AssetId) -> Result<Dependency, CookerError> {
    let source_url = source_id.to_url(&context.source_root)?;

    let (source, source_hash) = PipelineSource::load(&context.source_io, &source_url).await?;

    let dependencies = Arc::new(Mutex::new(Vec::new()));
    let cooked = source
        .cook(|sh, id| {
            let dependencies = dependencies.clone();
            let source_id = source_id.clone();
            async move {
                match sh {
                    ShaderType::Vertex => {
                        let vs_id = source_id
                            .create_relative(&id)
                            .map_err(|err| CookingError::from_err(source_id.as_str(), err))?;
                        let dep = cook_shader::cook_shader(context, vs_id)
                            .await
                            .map_err(|err| CookingError::from_err(source_id.as_str(), err))?;
                        let id = dep.cooked_id();
                        dependencies.lock().unwrap().push(dep);
                        Ok(id)
                    }

                    ShaderType::Fragment => {
                        let fs_id = source_id
                            .create_relative(&id)
                            .map_err(|err| CookingError::from_err(source_id.as_str(), err))?;
                        let dep = cook_shader::cook_shader(context, fs_id)
                            .await
                            .map_err(|err| CookingError::from_err(source_id.as_str(), err))?;
                        let id = dep.cooked_id();
                        dependencies.lock().unwrap().push(dep);
                        Ok(id)
                    }

                    _ => unreachable!(),
                }
            }
        })
        .await?;

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
            Arc::try_unwrap(dependencies)
                .map_err(|_| ())
                .unwrap()
                .into_inner()
                .unwrap(),
        )
        .await?)
}
