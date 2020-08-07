use crate::{
    assets::{AssetError, AssetIO, ShaderType, Url, UrlError},
    render::{Compile, CompiledShader, Context},
};
use shine_ecs::core::{
    observer::{Observable, Observer, SyncObserveDispatcher},
    store::{
        AsyncLoadHandler, AsyncLoader, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading, ReadGuard,
        Store,
    },
};
use std::{io, mem, pin::Pin, str::FromStr, sync::Arc};

/// Unique key for a shader
pub type ShaderKey = String;

#[derive(Debug, Clone)]
pub struct ShaderError;

pub enum ShaderEvent {
    Loaded,
}

pub struct Shader {
    id: String,
    shader: Result<Option<CompiledShader>, ShaderError>,
    dispatcher: SyncObserveDispatcher<ShaderEvent>,
}

impl Shader {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn shader(&self) -> Result<Option<&CompiledShader>, ShaderError> {
        match &self.shader {
            Err(_) => Err(ShaderError),
            Ok(None) => Ok(None),
            Ok(Some(shader)) => Ok(Some(shader)),
        }
    }

    pub fn shader_module(&self) -> Option<&CompiledShader> {
        self.shader.as_ref().map(|u| u.as_ref()).unwrap_or(None)
    }

    pub fn subscribe(&self, observer: &Arc<dyn Observer<ShaderEvent>>) {
        self.dispatcher.subscribe(observer)
    }

    pub fn unsubscribe(&self, observer: &Arc<dyn Observer<ShaderEvent>>) {
        self.dispatcher.unsubscribe(observer);
    }
}

impl Data for Shader {
    type Key = ShaderKey;
}

impl FromKey for Shader {
    fn from_key(key: &ShaderKey) -> Self {
        Shader {
            id: key.to_owned(),
            shader: Ok(None),
            dispatcher: Default::default(),
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
        log::trace!("[{:?}] Sending load request", load_token);
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
                self.shader = Err(ShaderError);
                log::warn!("[{:?}] Shader compilation failed: {:?}", load_token, err);
                self.dispatcher.notify_all(ShaderEvent::Loaded);
            }

            Ok((ty, spirv)) => {
                let shader = (ty, &spirv[..]).compile(context.device(), ());
                self.shader = Ok(Some(shader));
                log::debug!("[{:?}] Shader compilation completed", load_token);
                self.dispatcher.notify_all(ShaderEvent::Loaded);
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
        &self,
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

#[derive(Debug, Clone)]
pub struct ShaderDependencyError;

enum ShaderDependencyIndex {
    None,
    Pending(ShaderIndex),
    Completed(ShaderIndex),
    Error(ShaderDependencyError),
}

pub struct ShaderDependency {
    ty: Option<ShaderType>,
    id: Option<String>,
    index: ShaderDependencyIndex,
    requested_subscription: Option<Arc<dyn Observer<ShaderEvent>>>,
    active_subscription: Option<Arc<dyn Observer<ShaderEvent>>>,
}

impl ShaderDependency {
    pub fn none() -> ShaderDependency {
        ShaderDependency {
            ty: None,
            id: None,
            index: ShaderDependencyIndex::None,
            requested_subscription: None,
            active_subscription: None,
        }
    }

    pub fn with_type(self, ty: ShaderType) -> ShaderDependency {
        ShaderDependency {
            ty: Some(ty),
            index: ShaderDependencyIndex::None,
            active_subscription: None,
            ..self
        }
    }

    pub fn with_id<S: ToString>(self, id: S) -> ShaderDependency {
        ShaderDependency {
            id: Some(id.to_string()),
            index: ShaderDependencyIndex::None,
            active_subscription: None,
            ..self
        }
    }

    pub fn with_subscription<O>(self, observer: O) -> ShaderDependency
    where
        O: 'static + Observer<ShaderEvent>,
    {
        ShaderDependency {
            requested_subscription: Some(Arc::new(observer)),
            active_subscription: None,
            ..self
        }
    }

    pub fn request(&mut self, shaders: &mut ShaderStoreRead<'_>) {
        self.index = match mem::replace(&mut self.index, ShaderDependencyIndex::None) {
            ShaderDependencyIndex::None => {
                // create index from the shader key
                if self.ty.is_none() {
                    log::warn!("[{:?}] Missing shader type", self.id);
                    ShaderDependencyIndex::Error(ShaderDependencyError)
                } else if let Some(id) = &self.id {
                    ShaderDependencyIndex::Pending(shaders.get_or_load(id))
                } else {
                    log::warn!("[{:?}] Missing shader id or type", self.id);
                    ShaderDependencyIndex::Error(ShaderDependencyError)
                }
            }
            ShaderDependencyIndex::Pending(idx) => {
                // check if shader is loaded
                let shader = shaders.at(&idx);
                if let Some(sub) = self.requested_subscription.take() {
                    shader.subscribe(&sub);
                    self.active_subscription = Some(sub)
                }

                match shader.shader() {
                    Err(_) => {
                        log::warn!("[{:?}] Missing shader id or type", self.id);
                        ShaderDependencyIndex::Error(ShaderDependencyError)
                    }
                    Ok(None) => ShaderDependencyIndex::Pending(idx),
                    Ok(Some(st)) => {
                        if st.shader_type != self.ty.unwrap() {
                            ShaderDependencyIndex::Error(ShaderDependencyError)
                        } else {
                            log::info!("[{:?}] Shader dependency completed", self.id);
                            ShaderDependencyIndex::Completed(idx)
                        }
                    }
                }
            }
            ShaderDependencyIndex::Completed(idx) => ShaderDependencyIndex::Completed(idx),
            ShaderDependencyIndex::Error(err) => ShaderDependencyIndex::Error(err),
        }
    }

    pub fn shader_module<'m, 'a: 'm, 's: 'm>(
        &'s mut self,
        shaders: &'a ShaderStoreRead<'m>,
    ) -> Result<Option<&'m CompiledShader>, ShaderDependencyError> {
        match &self.index {
            ShaderDependencyIndex::None | ShaderDependencyIndex::Pending(_) => Ok(None),
            ShaderDependencyIndex::Error(err) => Err(err.clone()),
            ShaderDependencyIndex::Completed(idx) => Ok(shaders.at(idx).shader_module()),
        }
    }
}
