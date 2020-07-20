use crate::assets::{AssetError, AssetIO, TextureBuffer, TextureImage, Url, UrlError};
use crate::render::Context;
use shine_ecs::core::store::{
    AsyncLoadHandler, AsyncLoader, AutoNamedId, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading,
    ReadGuard, Store,
};
use std::pin::Pin;

/// Unique key for a texture
pub type TextureKey = String;

pub enum CompiledTexture {
    None,
    Error,
    Compiled(TextureBuffer),
}

pub struct Texture {
    id: String,
    texture: CompiledTexture,
}

impl Texture {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn texture(&self) -> &CompiledTexture {
        &self.texture
    }

    pub fn texture_buffer(&self) -> Option<&TextureBuffer> {
        if let CompiledTexture::Compiled(texture) = &self.texture {
            Some(texture)
        } else {
            None
        }
    }
}

impl Data for Texture {
    type Key = TextureKey;
}

impl FromKey for Texture {
    fn from_key(key: &TextureKey) -> Self {
        Texture {
            id: key.to_owned(),
            texture: CompiledTexture::None,
        }
    }
}

impl<'l> OnLoading<'l> for Texture {
    type LoadingContext = (&'l Context,);
}

impl OnLoad for Texture {
    type LoadRequest = TextureLoadRequest;
    type LoadResponse = TextureLoadResponse;
    type LoadHandler = AsyncLoadHandler<Self>;

    fn on_load_request(&mut self, load_handler: &mut Self::LoadHandler, load_token: LoadToken<Self>) {
        load_handler.request(load_token, TextureLoadRequest(self.id.clone()));
    }

    fn on_load_response<'l>(
        &mut self,
        _load_handler: &mut Self::LoadHandler,
        load_context: &mut (&'l Context,),
        load_token: LoadToken<Self>,
        load_response: TextureLoadResponse,
    ) {
        let (context,) = (load_context.0,);
        match load_response.0 {
            Err(err) => {
                self.texture = CompiledTexture::Error;
                //listeners.notify_all();
                log::warn!("[{:?}] Texture compilation failed: {:?}", load_token, err);
            }

            Ok(texture_image) => {
                match texture_image.to_texture_buffer(context.device()) {
                    Ok((texture, init_command)) => {
                        context.queue().submit(init_command);
                        self.texture = CompiledTexture::Compiled(texture);
                        //listeners.notify_all();
                        log::debug!("[{:?}] Texture compilation completed", load_token);
                    }
                    Err(err) => {
                        self.texture = CompiledTexture::Error;
                        //listeners.notify_all();
                        log::warn!("[{:?}] Texture compilation failed: {:?}", load_token, err);
                    }
                }
            }
        };
    }
}

pub struct TextureLoadRequest(TextureKey);
pub struct TextureLoadResponse(Result<TextureImage, TextureLoadError>);

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
    async fn load_texture(
        &self,
        load_token: LoadToken<Texture>,
        source_id: String,
    ) -> Result<TextureImage, TextureLoadError> {
        let url = Url::parse(&source_id)?;
        log::debug!("[{:?}] Loading texture...", load_token);
        let data = self.download_binary(&url).await?;

        load_token.ok()?;
        log::debug!("[{:?}] Decompressing texture...", load_token);
        let texture_image = bincode::deserialize::<TextureImage>(&data)?.decompress()?;
        Ok(texture_image)
    }
}

impl AsyncLoader<Texture> for AssetIO {
    fn load<'l>(
        &'l mut self,
        load_token: LoadToken<Texture>,
        request: TextureLoadRequest,
    ) -> Pin<Box<dyn 'l + std::future::Future<Output = Option<TextureLoadResponse>>>> {
        Box::pin(async move {
            match self.load_texture(load_token, request.0).await {
                Err(TextureLoadError::Canceled) => None,
                result => Some(TextureLoadResponse(result)),
            }
        })
    }
}

pub type TextureStore = Store<Texture, AsyncLoadHandler<Texture>>;
pub type TextureStoreRead<'a> = ReadGuard<'a, Texture, AsyncLoadHandler<Texture>>;
pub type TextureIndex = Index<Texture>;
pub type TextureNamedId = AutoNamedId<Texture>;

pub mod systems {
    use super::*;
    use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

    pub fn update_textures() -> Box<dyn Schedulable> {
        SystemBuilder::new("update_texture")
            .read_resource::<Context>()
            .write_resource::<TextureStore>()
            .build(move |_, _, (context, textures), _| {
                textures.load_and_finalize_requests((&*context,));
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
