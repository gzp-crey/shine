use crate::assets::{AssetError, AssetIO, ShaderType, Url, UrlError};
use crate::render::Context;
use shine_ecs::core::store::{
    AsyncLoadHandler, AsyncLoader, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading, ReadGuard, Store,
};
use std::pin::Pin;
use std::str::FromStr;
use std::{io, mem};

enum ShaderDependencyInner {
    Unknown,
    ShaderKey(ShaderType, ShaderKey),
    Pending(ShaderType, ShaderIndex),
    Subscribed(ShaderType, ShaderIndex),
    Completed(ShaderType, ShaderIndex),
    Failed,
}

impl ShaderDependencyInner {
    fn from_shader_index<F>(
        ty: ShaderType,
        id: ShaderIndex,
        shaders: &mut ShaderStoreRead<'_>,
        on_subscribe: F,
    ) -> ShaderDependencyInner
    where
        F: FnOnce(),
    {
        match shaders.at(&id) {
            Shader::Requested(_) => {
                on_subscribe();
                ShaderDependencyInner::Subscribed(ty, id)
            }
            Shader::Compiled(st, _) => {
                if *st == ty {
                    ShaderDependencyInner::Completed(ty, id)
                } else {
                    ShaderDependencyInner::Failed
                }
            }
            _ => ShaderDependencyInner::Failed,
        }
    }

    fn request<F>(self, shaders: &mut ShaderStoreRead<'_>, on_subscribe: F) -> ShaderDependencyInner
    where
        F: FnOnce(),
    {
        use ShaderDependencyInner::*;
        match self {
            s @ Completed(_, _) | s @ Failed | s @ Unknown => s,
            ShaderKey(ty, key) => {
                let id = shaders.get_or_add(&key.to_owned());
                ShaderDependencyInner::from_shader_index(ty, id, shaders, on_subscribe)
            }
            Pending(ty, id) => ShaderDependencyInner::from_shader_index(ty, id, shaders, on_subscribe),
            Subscribed(ty, id) => ShaderDependencyInner::from_shader_index(ty, id, shaders, || {}),
        }
    }
}


/// Error indicating a failed shader dependency request.
pub struct ShaderDependencyError;

/// Helper to manage dependency on a shader
pub struct ShaderDependency(ShaderDependencyInner);

impl ShaderDependency {
    pub fn unknown() -> ShaderDependency {
        ShaderDependency(ShaderDependencyInner::Unknown)
    }

    pub fn from_key(ty: ShaderType, key: ShaderKey) -> ShaderDependency {
        ShaderDependency(ShaderDependencyInner::ShaderKey(ty, key))
    }

    pub fn from_index(ty: ShaderType, id: ShaderIndex) -> ShaderDependency {
        ShaderDependency(ShaderDependencyInner::Pending(ty, id))
    }

    pub fn request<F>(&mut self, shaders: &mut ShaderStoreRead<'_>, on_subscribe: F) -> Result<Option<&ShaderIndex>, ShaderDependencyError>
    where
        F: FnOnce(),
    {
        self.0 = mem::replace(&mut self.0, ShaderDependencyInner::Failed).request(shaders, on_subscribe);
        match &self.0 {
            Completed(_, id) => Ok(Some(id)),
            Failed => Err(ShaderDependencyError),
            _ => Ok(None)
        }
    }
}

/// Unique key for a shader
pub type ShaderKey = String;

pub enum Shader {
    Requested(Url),
    Compiled(ShaderType, wgpu::ShaderModule),
    Error,
}

impl Shader {
    pub fn shadere_module(&self) -> Option<&wgpu::ShaderModule> {
        if let Shader::Compiled(_, ref sh) = self {
            Some(sh)
        } else {
            None
        }
    }
}

impl Data for Shader {
    type Key = ShaderKey;
}

impl FromKey for Shader {
    fn from_key(key: &ShaderKey) -> Self {
        match Url::parse(&key) {
            Ok(url) => Shader::Requested(url /*, LoadListener::new()*/),
            Err(err) => {
                log::warn!("Invalid shader url ({}): {:?}", key, err);
                Shader::Error
            }
        }
    }
}

impl<'a> OnLoading<'a> for Shader {
    type LoadingContext = &'a Context;
}

impl OnLoad for Shader {
    type LoadRequest = ShaderLoadRequest;
    type LoadResponse = ShaderLoadResponse;
    type LoadHandler = AsyncLoadHandler<Self>;

    fn on_load_request(&mut self, load_handler: &mut Self::LoadHandler, load_token: LoadToken<Self>) {
        match self {
            Shader::Requested(id) => load_handler.request(load_token, id.clone()),
            _ => unreachable!(),
        }
    }

    fn on_load_response<'l>(
        &mut self,
        _load_handler: &mut Self::LoadHandler,
        load_context: &mut &'l Context,
        load_token: LoadToken<Self>,
        load_response: ShaderLoadResponse,
    ) {
        *self = match (mem::replace(self, Shader::Error), load_response) {
            (Shader::Requested(_ /*, listeners*/), Err(err)) => {
                //listeners.notify_all();
                log::warn!("[{:?}] Shader compilation failed: {:?}", load_token, err);
                Shader::Error
            }

            (Shader::Requested(_ /*, listeners*/), Ok((ty, spirv))) => {
                //listeners.notify_all();
                let shader = load_context
                    .device()
                    .create_shader_module(wgpu::util::make_spirv(&spirv));
                log::debug!("[{:?}] Shader compilation completed", load_token);
                Shader::Compiled(ty, shader)
            }

            _ => unreachable!(),
        };
    }
}

pub type ShaderLoadRequest = Url;
pub type ShaderLoadResponse = Result<(ShaderType, Vec<u8>), ShaderLoadError>;

/// Error during shader load
#[derive(Debug)]
pub enum ShaderLoadError {
    Asset(AssetError),
    Canceled,
}

impl From<UrlError> for ShaderLoadError {
    fn from(err: UrlError) -> ShaderLoadError {
        ShaderLoadError::Asset(AssetError::InvalidUrl(err))
    }
}

impl From<LoadCanceled> for ShaderLoadError {
    fn from(_err: LoadCanceled) -> ShaderLoadError {
        ShaderLoadError::Canceled
    }
}

impl From<AssetError> for ShaderLoadError {
    fn from(err: AssetError) -> ShaderLoadError {
        ShaderLoadError::Asset(err)
    }
}

impl From<io::Error> for ShaderLoadError {
    fn from(err: io::Error) -> ShaderLoadError {
        ShaderLoadError::Asset(AssetError::ContentLoad(format!("{:?}", err)))
    }
}

impl AssetIO {
    async fn load_shader_from_url(&mut self, load_token: LoadToken<Shader>, url: Url) -> ShaderLoadResponse {
        let ty = ShaderType::from_str(url.extension())?;
        log::debug!("[{:?}] Loading shader...", load_token);
        let data = self.download_binary(&url).await?;
        Ok((ty, data))
    }
}

impl AsyncLoader<Shader> for AssetIO {
    fn load<'a>(
        &'a mut self,
        load_token: LoadToken<Shader>,
        request: ShaderLoadRequest,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<ShaderLoadResponse>>>> {
        Box::pin(async move {
            match self.load_shader_from_url(load_token, request).await {
                Err(ShaderLoadError::Canceled) => None,
                result => Some(result),
            }
        })
    }
}

pub type ShaderStore = Store<Shader, AsyncLoadHandler<Shader>>;
pub type ShaderStoreRead<'a> = ReadGuard<'a, Shader, AsyncLoadHandler<Shader>>;
pub type ShaderIndex = Index<Shader>;

pub mod systems {
    use super::*;
    use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

    pub fn update_shaders() -> Box<dyn Schedulable> {
        SystemBuilder::new("update_shaders")
            .read_resource::<Context>()
            .write_resource::<ShaderStore>()
            .build(move |_, _, (context, shaders), _| {
                shaders.load_and_finalize_requests(&*context);
            })
    }

    pub fn gc_shaders() -> Box<dyn Schedulable> {
        SystemBuilder::new("gc_shaders")
            .write_resource::<ShaderStore>()
            .build(move |_, _, shaders, _| {
                shaders.drain_unused();
            })
    }
}
