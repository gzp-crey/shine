use crate::{
    assets::{AssetError, AssetIO, TextureImage, Url, UrlError},
    render::{Compile, CompiledTexture, Context},
};
use shine_ecs::core::{
    observer::{ObserveDispatcher, Subscription},
    store::{
        AsyncLoadHandler, AsyncLoader, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading, ReadGuard,
        Store,
    },
};
use std::{mem, pin::Pin};

/// Unique key for a texture
pub type TextureKey = String;

#[derive(Debug, Clone)]
pub struct TextureError;

pub enum TextureEvent {
    Loaded,
}

pub struct Texture {
    id: String,
    texture: Result<Option<CompiledTexture>, TextureError>,
    dispatcher: ObserveDispatcher<TextureEvent>,
}

impl Texture {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn dispatcher(&self) -> &ObserveDispatcher<TextureEvent> {
        &self.dispatcher
    }

    pub fn texture(&self) -> Result<Option<&CompiledTexture>, TextureError> {
        match &self.texture {
            Err(_) => Err(TextureError),
            Ok(None) => Ok(None),
            Ok(Some(texture)) => Ok(Some(texture)),
        }
    }

    pub fn texture_buffer(&self) -> Option<&CompiledTexture> {
        if let Ok(Some(texture)) = &self.texture {
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
            texture: Ok(None),
            dispatcher: Default::default(),
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
                self.texture = Err(TextureError);
                self.dispatcher.notify_all(TextureEvent::Loaded);
                log::warn!("[{:?}] Texture compilation failed: {:?}", load_token, err);
            }

            Ok(texture_image) => match texture_image.compile(context.device(), ()) {
                Ok((texture, init_command)) => {
                    context.queue().submit(init_command);
                    self.texture = Ok(Some(texture));
                    self.dispatcher.notify_all(TextureEvent::Loaded);
                    log::debug!("[{:?}] Texture compilation completed", load_token);
                }
                Err(err) => {
                    self.texture = Err(TextureError);
                    self.dispatcher.notify_all(TextureEvent::Loaded);
                    log::warn!("[{:?}] Texture compilation failed: {:?}", load_token, err);
                }
            },
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

#[derive(Debug, Clone)]
pub struct TextureDependencyError;

enum TextureDependencyIndex {
    None,
    Incomplete,
    Pending(TextureIndex, Option<Subscription<TextureEvent>>),
    Completed(TextureIndex, Option<Subscription<TextureEvent>>),
    Error(TextureDependencyError),
}
pub struct TextureDependency {
    id: Option<String>,
    index: TextureDependencyIndex,
}

impl TextureDependency {
    pub fn none() -> TextureDependency {
        TextureDependency {
            id: None,
            index: TextureDependencyIndex::None,
        }
    }

    pub fn new() -> TextureDependency {
        TextureDependency {
            id: None,
            index: TextureDependencyIndex::Incomplete,
        }
    }

    pub fn with_id<S: ToString>(self, id: S) -> TextureDependency {
        assert!(self.is_none());
        TextureDependency {
            id: Some(id.to_string()),
            index: TextureDependencyIndex::Incomplete,
            ..self
        }
    }

    pub fn or_id<S: ToString>(&mut self, id: S) -> &mut TextureDependency {
        assert!(self.is_none());
        if self.id.is_none() {
            self.id = Some(id.to_string());
            self.index = TextureDependencyIndex::Incomplete;
        }
        self
    }

    pub fn is_none(&self) -> bool {
        if let TextureDependencyIndex::None = self.index {
            true
        } else {
            false
        }
    }

    pub fn key(&self) -> Result<TextureKey, TextureDependencyError> {
        if self.id.is_none() {
            log::warn!("Missing textur id");
            Err(TextureDependencyError)
        } else {
            Ok(self.id.clone().unwrap())
        }
    }

    pub fn compiled_texture<'c, 'r: 'c, 's: 'c>(
        &'s mut self,
        textures: &'r TextureStoreRead<'c>,
    ) -> Result<Option<&'c CompiledTexture>, TextureDependencyError> {
        match &self.index {
            TextureDependencyIndex::None | TextureDependencyIndex::Incomplete => Ok(None),
            TextureDependencyIndex::Pending(_, _) => Ok(None),
            TextureDependencyIndex::Error(err) => Err(err.clone()),
            TextureDependencyIndex::Completed(idx, _) => Ok(textures.at(idx).texture_buffer()),
        }
    }

    pub fn request_with<'c, 'r: 'c, 's: 'c, S>(
        &'s mut self,
        textures: &'r TextureStoreRead<'c>,
        subscription: S,
    ) -> Result<Option<&'c CompiledTexture>, TextureDependencyError>
    where
        S: FnOnce(&Texture) -> Option<Subscription<TextureEvent>>,
    {
        self.index = match mem::replace(&mut self.index, TextureDependencyIndex::Incomplete) {
            TextureDependencyIndex::Incomplete => match self.key() {
                Err(err) => TextureDependencyIndex::Error(err),
                Ok(id) => {
                    let idx = textures.get_or_load(&id);
                    let sub = subscription(&textures.at(&idx));
                    TextureDependencyIndex::Pending(idx, sub)
                }
            },
            TextureDependencyIndex::Pending(idx, sub) => {
                let texture = textures.at(&idx);
                //self.subscription.subscribe(&texture.dispatcher);

                match texture.texture() {
                    Err(_) => TextureDependencyIndex::Error(TextureDependencyError),
                    Ok(None) => TextureDependencyIndex::Pending(idx, sub),
                    Ok(Some(_)) => TextureDependencyIndex::Completed(idx, sub),
                }
            }
            TextureDependencyIndex::None => TextureDependencyIndex::None,
            TextureDependencyIndex::Completed(idx, sub) => TextureDependencyIndex::Completed(idx, sub),
            TextureDependencyIndex::Error(err) => TextureDependencyIndex::Error(err),
        };

        self.compiled_texture(textures)
    }

    pub fn request<'c, 'r: 'c, 's: 'c>(
        &'s mut self,
        textures: &'r TextureStoreRead<'c>,
    ) -> Result<Option<&'c CompiledTexture>, TextureDependencyError> {
        self.request_with(textures, |_| None)
    }
}

impl Default for TextureDependency {
    fn default() -> Self {
        Self::new()
    }
}
