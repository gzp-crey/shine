use crate::assets::{AssetError, AssetIO, TextureBuffer, TextureImage, Url, UrlError};
use crate::render::Context;
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, DataUpdater, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::pin::Pin;
use std::sync::Arc;

/// Error during texture loading
#[derive(Debug)]
pub enum TextureLoadError {
    Asset(AssetError),
    Canceled,
}

impl From<UrlError> for TextureLoadError {
    fn from(err: UrlError) -> TextureLoadError {
        TextureLoadError::Asset(AssetError::InvalidUrl(err))
    }
}

impl From<AssetError> for TextureLoadError {
    fn from(err: AssetError) -> TextureLoadError {
        TextureLoadError::Asset(err)
    }
}

impl From<bincode::Error> for TextureLoadError {
    fn from(err: bincode::Error) -> TextureLoadError {
        TextureLoadError::Asset(AssetError::ContentLoad(format!("Binary stream error: {}", err)))
    }
}

pub enum Texture {
    Pending(LoadListeners),
    Compiled(TextureBuffer),
    Error,
    None,
}

impl Texture {
    pub fn texture_buffer(&self) -> Option<&TextureBuffer> {
        if let Texture::Compiled(ref texture_buffer) = self {
            Some(texture_buffer)
        } else {
            None
        }
    }

    fn on_update(
        &mut self,
        load_context: LoadContext<'_, Texture>,
        context: &Context,
        load_response: TextureLoadResponse,
    ) -> Option<String> {
        *self = match (std::mem::replace(self, Texture::None), load_response) {
            (Texture::Pending(listeners), Err(err)) => {
                listeners.notify_all();
                log::debug!("Texture[{:?}] compilation failed: {:?}", load_context, err);
                Texture::Error
            }

            (Texture::Pending(listeners), Ok(texture_image)) => {
                listeners.notify_all();
                match texture_image.to_texture_buffer(context.device()) {
                    Ok((texture_buffer, init_command)) => {
                        context.queue().submit(Some(init_command));
                        log::debug!("Texture[{:?}] compilation completed", load_context);
                        Texture::Compiled(texture_buffer)
                    }
                    Err(err) => {
                        log::debug!("Texture[{:?}] compilation failed: {:?}", load_context, err);
                        Texture::Error
                    }
                }
            }

            (Texture::Compiled(_), _) => unreachable!(),
            (Texture::Error, _) => unreachable!(),
            (Texture::None, _) => unreachable!(),
        };
        None
    }
}

impl Data for Texture {
    type Key = String;
    type LoadRequest = TextureLoadRequest;
    type LoadResponse = TextureLoadResponse;
}

impl FromKey for Texture {
    fn from_key(key: &String) -> (Self, Option<String>) {
        (Texture::Pending(LoadListeners::new()), Some(key.to_owned()))
    }
}

pub type TextureLoadRequest = String;
pub type TextureLoadResponse = Result<TextureImage, TextureLoadError>;

pub struct TextureLoader {
    base_url: Url,
    assetio: Arc<AssetIO>,
}

impl TextureLoader {
    pub fn new(assetio: Arc<AssetIO>, base_url: Url) -> TextureLoader {
        TextureLoader { base_url, assetio }
    }

    async fn load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Texture>,
        source_id: String,
    ) -> TextureLoadResponse {
        if cancellation_token.is_canceled() {
            return Err(TextureLoadError::Canceled);
        }

        let url = self.base_url.join(&source_id)?;

        log::debug!("[{}] Loading texture...", url.as_str());
        let data = self.assetio.download_binary(&url).await?;

        log::debug!("[{}] Decompressiong texture...", url.as_str());
        let texture_image = bincode::deserialize::<TextureImage>(&data)?.decompress()?;
        Ok(texture_image)
    }

    async fn try_load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Texture>,
        source_id: String,
    ) -> Option<TextureLoadResponse> {
        match self.load_from_url(cancellation_token, source_id).await {
            Err(TextureLoadError::Canceled) => None,
            result => Some(result),
        }
    }
}

impl DataLoader<Texture> for TextureLoader {
    fn load<'a>(
        &'a mut self,
        source_id: String,
        cancellation_token: CancellationToken<Texture>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<TextureLoadResponse>>>> {
        Box::pin(self.try_load_from_url(cancellation_token, source_id))
    }
}

pub struct TextureUpdater<'a>(&'a Context);

impl<'a> DataUpdater<'a, Texture> for TextureUpdater<'a> {
    fn update<'u>(
        &mut self,
        load_context: LoadContext<'u, Texture>,
        data: &mut Texture,
        load_response: TextureLoadResponse,
    ) -> Option<TextureLoadRequest> {
        data.on_update(load_context, self.0, load_response)
    }
}

pub type TextureStore = Store<Texture>;
pub type TextureStoreRead<'a> = ReadGuard<'a, Texture>;
pub type TextureIndex = Index<Texture>;

pub mod systems {
    use super::*;
    use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

    pub fn update_textures() -> Box<dyn Schedulable> {
        SystemBuilder::new("update_texture")
            .read_resource::<Context>()
            .write_resource::<TextureStore>()
            .build(move |_, _, (context, textures), _| {
                let mut textures = textures.write();
                let context: &Context = &*context;
                //shaders.drain_unused();
                textures.update(&mut TextureUpdater(context));
                textures.finalize_requests();
            })
    }
}
