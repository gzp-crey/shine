use crate::assets::{AssetError, AssetIO, TextureBuffer, TextureImage, Url, UrlError};
use crate::render::Context;
use shine_ecs::core::store::{
    AsyncLoadHandler, AsyncLoader, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading, ReadGuard, Store,
};
use std::pin::Pin;

/// Unique key for a texture
pub type TextureKey = String;

pub enum Texture {
    Requested(Url /*LoadListeners*/),
    Compiled(TextureBuffer),
    Error,
}

impl Texture {
    pub fn texture_buffer(&self) -> Option<&TextureBuffer> {
        if let Texture::Compiled(ref texture_buffer) = self {
            Some(texture_buffer)
        } else {
            None
        }
    }
}

impl Data for Texture {
    type Key = TextureKey;
}

impl FromKey for Texture {
    fn from_key(key: &String) -> Self {
        match Url::parse(&key) {
            Ok(url) => Texture::Requested(url /*, LoadListener::new()*/),
            Err(err) => {
                log::warn!("Invalid texture url ({}): {:?}", key, err);
                Texture::Error
            }
        }
    }
}

impl<'a> OnLoading<'a> for Texture {
    type LoadingContext = &'a Context;
}

impl OnLoad for Texture {
    type LoadRequest = TextureLoadRequest;
    type LoadResponse = TextureLoadResponse;
    type LoadHandler = AsyncLoadHandler<Self>;

    fn on_load_request(&mut self, load_handler: &mut Self::LoadHandler, load_token: LoadToken<Self>) {
        match self {
            Texture::Requested(id) => load_handler.request(load_token, id.to_owned()),
            _ => unreachable!(),
        }
    }

    fn on_load_response<'l>(
        &mut self,
        _load_handler: &mut Self::LoadHandler,
        load_context: &mut &'l Context,
        load_token: LoadToken<Self>,
        load_response: TextureLoadResponse,
    ) {
        *self = match (std::mem::replace(self, Texture::Error), load_response) {
            (Texture::Requested(_ /*,listeners*/), Err(err)) => {
                //listeners.notify_all();
                log::warn!("[{:?}] Texture compilation failed: {:?}", load_token, err);
                Texture::Error
            }

            (Texture::Requested(_ /*,listeners*/), Ok(texture_image)) => {
                //listeners.notify_all();
                match texture_image.to_texture_buffer(load_context.device()) {
                    Ok((texture_buffer, init_command)) => {
                        load_context.queue().submit(init_command);
                        log::debug!("[{:?}] Texture compilation completed", load_token);
                        Texture::Compiled(texture_buffer)
                    }
                    Err(err) => {
                        log::warn!("[{:?}] Texture compilation failed: {:?}", load_token, err);
                        Texture::Error
                    }
                }
            }

            _ => unreachable!(),
        };
    }
}

pub type TextureLoadRequest = Url;
pub type TextureLoadResponse = Result<TextureImage, TextureLoadError>;

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

impl From<LoadCanceled> for TextureLoadError {
    fn from(_err: LoadCanceled) -> TextureLoadError {
        TextureLoadError::Canceled
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

impl AssetIO {
    async fn load_texture_from_url(&mut self, load_token: LoadToken<Texture>, url: Url) -> TextureLoadResponse {
        log::debug!("[{:?}] Loading texture...", load_token);
        let data = self.download_binary(&url).await?;

        load_token.ok()?;
        log::debug!("[{:?}] Decompressing texture...", load_token);
        let texture_image = bincode::deserialize::<TextureImage>(&data)?.decompress()?;
        Ok(texture_image)
    }
}

impl AsyncLoader<Texture> for AssetIO {
    fn load<'a>(
        &'a mut self,
        load_token: LoadToken<Texture>,
        request: TextureLoadRequest,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<TextureLoadResponse>>>> {
        Box::pin(async move {
            match self.load_texture_from_url(load_token, request).await {
                Err(TextureLoadError::Canceled) => None,
                result => Some(result),
            }
        })
    }
}

pub type TextureStore = Store<Texture, AsyncLoadHandler<Texture>>;
pub type TextureStoreRead<'a> = ReadGuard<'a, Texture, AsyncLoadHandler<Texture>>;
pub type TextureIndex = Index<Texture>;

pub mod systems {
    use super::*;
    use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

    pub fn update_textures() -> Box<dyn Schedulable> {
        SystemBuilder::new("update_texture")
            .read_resource::<Context>()
            .write_resource::<TextureStore>()
            .build(move |_, _, (context, textures), _| {
                textures.load_and_finalize_requests(&*context);
            })
    }

    pub fn gc_textures() -> Box<dyn Schedulable> {
        SystemBuilder::new("gc_texture")
            .write_resource::<TextureStore>()
            .build(move |_, _, textures, _| {
                textures.drain_unused();
            })
    }
}
