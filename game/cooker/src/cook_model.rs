use crate::Context;
use shine_game::assets::{
    cooker::{CookingError, ModelCooker, Naming},
    AssetError, AssetId, GltfSource, Url,
};
use std::{future::Future, pin::Pin};

impl<'a> ModelCooker<'a> for Context {
    type ModelFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_model(&self, source_id: AssetId, naming: Naming) -> Self::ModelFuture {
        Box::pin({
            let context = self.clone();
            async move {
                let source_url = source_id
                    .to_url(&context.source_root)
                    .map_err(|err| CookingError::from_err(&source_id, err))?;

                let ext = source_url.extension();
                let (cooked, source_hash) = match ext {
                    "gltf" | "glb" => {
                        let (source, source_hash) = GltfSource::load(&context.source_io, &source_id, &source_url)
                            .await
                            .map_err(|err| CookingError::from_err(&source_id, err))?;
                        (source.cook().await?, source_hash)
                    }

                    ext => {
                        return Err(CookingError::from_err(
                            &source_id,
                            AssetError::UnsupportedFormat(ext.to_owned()),
                        ));
                    }
                };
                let cooked_content =
                    bincode::serialize(&cooked).map_err(|err| CookingError::from_err(&source_id, err))?;

                log::debug!("[{}] Uploading...", source_url);
                let cooked_url = context
                    .target_io
                    .upload_binary_content(source_id, source_hash, naming, &cooked_content)
                    .await?;

                Ok(cooked_url)
            }
        })
    }
}
