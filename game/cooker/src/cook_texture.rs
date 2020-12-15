use crate::Context;
use shine_game::assets::{
    cooker::{CookingError, Naming, TextureCooker},
    AssetId, TextureSource, Url,
};
use std::{future::Future, pin::Pin};

impl<'a> TextureCooker<'a> for Context {
    type TextureFuture = Pin<Box<dyn Future<Output = Result<Url, CookingError>>>>;

    fn cook_texture(&self, source_id: AssetId, naming: Naming) -> Self::TextureFuture {
        Box::pin({
            let context = self.clone();
            async move {
                let source_url = source_id
                    .to_url(&context.source_root)
                    .map_err(|err| CookingError::from_err(source_id.to_string(), err))?;
                let (source, source_hash) = TextureSource::load(&context.source_io, &source_id, &source_url)
                    .await
                    .map_err(|err| CookingError::from_err(source_id.to_string(), err))?;

                let cooked = source.cook().await?;
                let cooked_content =
                    bincode::serialize(&cooked).map_err(|err| CookingError::from_err(source_id.to_string(), err))?;

                log::debug!("[{}] Uploading...", source_url.as_str());
                let cooked_url = context
                    .target_io
                    .upload_binary_content(source_id, source_hash, naming, &cooked_content)
                    .await?;

                Ok(cooked_url)
            }
        })
    }
}
