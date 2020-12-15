use crate::Context;
use shine_game::assets::{
    cooker::{CookingError, Naming, ShaderCooker},
    AssetId, ShaderSource, Url,
};
use std::{future::Future, pin::Pin};

impl<'a> ShaderCooker<'a> for Context {
    type ShaderFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_shader(&self, source_id: AssetId, naming: Naming) -> Self::ShaderFuture {
        Box::pin({
            let context = self.clone();
            async move {
                let source_url = source_id
                    .to_url(&context.source_root)
                    .map_err(|err| CookingError::from_err(&source_id, err))?;
                let (source, source_hash) = ShaderSource::load(&context.source_io, &source_id, &source_url)
                    .await
                    .map_err(|err| CookingError::from_err(&source_id, err))?;

                let cooked = source.cook().await?;
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
