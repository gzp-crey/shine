use crate::assets::{AssetError, AssetIO, ShaderType, Url, UrlError};
use crate::render::Context;
use shine_ecs::core::store::{
    AsyncLoadHandler, AsyncLoadListeners, AsyncLoader, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad,
    OnLoading, ReadGuard, Store,
};
use std::pin::Pin;
use std::str::FromStr;
use std::{io, mem};

/// Unique key for a shader
pub type ShaderKey = String;

pub enum CompiledShader {
    None,
    Error,
    Compiled(ShaderType, wgpu::ShaderModule),
}

pub struct Shader {
    id: String,
    shader: CompiledShader,
    listeners: AsyncLoadListeners,
}

impl Shader {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn shader(&self) -> &CompiledShader {
        &self.shader
    }

    pub fn shader_module(&self) -> Option<&wgpu::ShaderModule> {
        if let CompiledShader::Compiled(_, sh) = &self.shader {
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
        Shader {
            id: key.to_owned(),
            shader: CompiledShader::None,
            listeners: AsyncLoadListeners::default(),
        }
    }
}

impl<'l> OnLoading<'l> for Shader {
    type LoadingContext = (&'l Context,);
}

impl OnLoad for Shader {
    type LoadRequest = ShaderLoadRequest;
    type LoadResponse = ShaderLoadResponse;
    type LoadHandler = AsyncLoadHandler<Self>;

    fn on_load_request(&mut self, load_handler: &mut Self::LoadHandler, load_token: LoadToken<Self>) {
        load_handler.request(load_token, ShaderLoadRequest(self.id.clone()));
    }

    fn on_load_response<'l>(
        &mut self,
        _load_handler: &mut Self::LoadHandler,
        load_context: &mut (&'l Context,),
        load_token: LoadToken<Self>,
        load_response: ShaderLoadResponse,
    ) {
        let (context,) = (load_context.0,);
        match load_response.0 {
            Err(err) => {
                self.shader = CompiledShader::Error;
                log::warn!("[{:?}] Shader compilation failed: {:?}", load_token, err);
                self.listeners.notify_all();
            }

            Ok((ty, spirv)) => {
                let shader = context.device().create_shader_module(wgpu::util::make_spirv(&spirv));
                self.shader = CompiledShader::Compiled(ty, shader);
                log::debug!("[{:?}] Shader compilation completed", load_token);
                self.listeners.notify_all();
            }
        };
    }
}

pub struct ShaderLoadRequest(ShaderKey);
pub struct ShaderLoadResponse(Result<(ShaderType, Vec<u8>), ShaderLoadError>);

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
    async fn load_shader(
        &mut self,
        load_token: LoadToken<Shader>,
        source_id: String,
    ) -> Result<(ShaderType, Vec<u8>), ShaderLoadError> {
        let url = Url::parse(&source_id)?;
        let ty = ShaderType::from_str(url.extension())?;
        log::debug!("[{:?}] Loading shader...", load_token);
        let data = self.download_binary(&url).await?;
        Ok((ty, data))
    }
}

impl AsyncLoader<Shader> for AssetIO {
    fn load<'l>(
        &'l mut self,
        load_token: LoadToken<Shader>,
        request: ShaderLoadRequest,
    ) -> Pin<Box<dyn 'l + std::future::Future<Output = Option<ShaderLoadResponse>>>> {
        Box::pin(async move {
            match self.load_shader(load_token, request.0).await {
                Err(ShaderLoadError::Canceled) => None,
                result => Some(ShaderLoadResponse(result)),
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
                shaders.load_and_finalize_requests((&*context,));
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

enum ShaderDependencyInner {
    Unknown,
    None,
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
        F: FnOnce(&AsyncLoadListeners),
    {
        let shader = shaders.at(&id);
        match &shader.shader {
            CompiledShader::None => {
                on_subscribe(&shader.listeners);
                ShaderDependencyInner::Subscribed(ty, id)
            }
            CompiledShader::Compiled(st, _) => {
                if *st == ty {
                    ShaderDependencyInner::Completed(ty, id)
                } else {
                    ShaderDependencyInner::Failed
                }
            }
            CompiledShader::Error => ShaderDependencyInner::Failed,
        }
    }

    fn request<F>(self, shaders: &mut ShaderStoreRead<'_>, on_subscribe: F) -> ShaderDependencyInner
    where
        F: FnOnce(&AsyncLoadListeners),
    {
        use ShaderDependencyInner::*;
        match self {
            s @ Completed(_, _) | s @ Failed | s @ Unknown | s @ None => s,
            ShaderKey(ty, key) => {
                let id = shaders.get_or_add(&key);
                ShaderDependencyInner::from_shader_index(ty, id, shaders, on_subscribe)
            }
            Pending(ty, id) => ShaderDependencyInner::from_shader_index(ty, id, shaders, on_subscribe),
            Subscribed(ty, id) => ShaderDependencyInner::from_shader_index(ty, id, shaders, |_| {}),
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

    pub fn none() -> ShaderDependency {
        ShaderDependency(ShaderDependencyInner::None)
    }

    pub fn from_key(ty: ShaderType, key: ShaderKey) -> ShaderDependency {
        ShaderDependency(ShaderDependencyInner::ShaderKey(ty, key))
    }

    pub fn from_index(ty: ShaderType, id: ShaderIndex) -> ShaderDependency {
        ShaderDependency(ShaderDependencyInner::Pending(ty, id))
    }

    pub fn request<F>(&mut self, shaders: &mut ShaderStoreRead<'_>, on_subscribe: F)
    where
        F: FnOnce(&AsyncLoadListeners),
    {
        self.0 = mem::replace(&mut self.0, ShaderDependencyInner::Failed).request(shaders, on_subscribe);
    }

    pub fn shader_module<'m, 's: 'm, 'a: 'm>(
        &'s mut self,
        shaders: &'a ShaderStoreRead<'m>,
    ) -> Result<Option<&'m wgpu::ShaderModule>, ShaderDependencyError> {
        match &self.0 {
            ShaderDependencyInner::Completed(_, id) => Ok(shaders.at(id).shader_module()),
            ShaderDependencyInner::Failed => Err(ShaderDependencyError),
            _ => Ok(None),
        }
    }
}
