use crate::utils::{assets, url::Url};
use crate::{
    render::{Context, TextureBuffer, TextureImage},
    GameError,
};
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, DataUpdater, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::pin::Pin;

pub enum Texture {
    Pending(LoadListeners),
    Compiled(TextureBuffer),
    Error(String),
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
            (Texture::Pending(listeners), TextureLoadResponse::Error(err)) => {
                log::debug!("Texture compilation failed [{:?}]: {:?}", load_context, err);
                listeners.notify_all();
                Texture::Error(err)
            }

            (Texture::Pending(listeners), TextureLoadResponse::TextureImage(texture_image)) => {
                log::debug!("Texture compilation completed for [{:?}]", load_context);
                listeners.notify_all();
                Texture::Compiled(texture_image.to_texture_buffer(context.device()))
            }

            (Texture::Compiled(_), _) => unreachable!(),
            (Texture::Error(_), _) => unreachable!(),
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

pub enum TextureLoadResponse {
    TextureImage(TextureImage),
    Error(String),
}

pub struct TextureLoader {
    base_url: Url,
}

impl TextureLoader {
    pub fn new(base_url: &str) -> Result<TextureLoader, GameError> {
        let base_url = Url::parse(base_url)
            .map_err(|err| GameError::Config(format!("Failed to parse base url for texture: {:?}", err)))?;

        Ok(TextureLoader { base_url })
    }

    async fn load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Texture>,
        source_id: String,
    ) -> Option<TextureLoadResponse> {
        if cancellation_token.is_canceled() {
            return None;
        }

        let url = match self.base_url.join(&source_id) {
            Err(err) => {
                let err = format!("Invalid texture url ({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(TextureLoadResponse::Error(err));
            }
            Ok(url) => url,
        };

        log::debug!("Texture loading: [{}]", url.as_str());
        let data = match assets::download_binary(&url).await {
            Err(err) => {
                let err = format!("Failed to get texture({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(TextureLoadResponse::Error(err));
            }
            Ok(data) => data,
        };

        let texture_image = match bincode::deserialize::<TextureImage>(&data) {
            Err(err) => {
                let err = format!("Failed to parse texture({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(TextureLoadResponse::Error(err));
            }
            Ok(texture_image) => texture_image,
        };

        let texture_image = texture_image.decompress();
        Some(TextureLoadResponse::TextureImage(texture_image))
    }
}

impl DataLoader<Texture> for TextureLoader {
    fn load<'a>(
        &'a mut self,
        source_id: String,
        cancellation_token: CancellationToken<Texture>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<TextureLoadResponse>>>> {
        Box::pin(self.load_from_url(cancellation_token, source_id))
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
