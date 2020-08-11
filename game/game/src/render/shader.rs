use crate::{
    assets::{AssetError, AssetIO, ShaderType, Url, UrlError},
    render::{Compile, CompiledShader, Context},
};
use shine_ecs::core::{
    observer::{ObserveDispatcher, ObserveResult, Observer, SubscriptionRequest},
    store::{
        AsyncLoadHandler, AsyncLoader, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading, ReadGuard,
        Store,
    },
};
use std::{io, mem, pin::Pin, str::FromStr};

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
    dispatcher: ObserveDispatcher<ShaderEvent>,
}

impl Shader {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn dispatcher(&self) -> &ObserveDispatcher<ShaderEvent> {
        &self.dispatcher
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
    Incomplete,
    Pending(ShaderIndex),
    Completed(ShaderIndex),
    Error(ShaderDependencyError),
}
pub struct ShaderDependency {
    ty: Option<ShaderType>,
    id: Option<String>,
    index: ShaderDependencyIndex,
    subscription: SubscriptionRequest<ShaderEvent>,
}

impl ShaderDependency {
    pub fn none() -> ShaderDependency {
        ShaderDependency {
            ty: None,
            id: None,
            index: ShaderDependencyIndex::None,
            subscription: SubscriptionRequest::None,
        }
    }

    pub fn new() -> ShaderDependency {
        ShaderDependency {
            ty: None,
            id: None,
            index: ShaderDependencyIndex::Incomplete,
            subscription: SubscriptionRequest::None,
        }
    }

    pub fn with_type(self, ty: ShaderType) -> ShaderDependency {
        assert!(self.is_none());
        ShaderDependency {
            ty: Some(ty),
            index: ShaderDependencyIndex::Incomplete,
            subscription: self.subscription.with_cancel_active(),
            ..self
        }
    }

    pub fn with_id<S: ToString>(self, id: S) -> ShaderDependency {
        assert!(self.is_none());
        ShaderDependency {
            id: Some(id.to_string()),
            index: ShaderDependencyIndex::Incomplete,
            subscription: self.subscription.with_cancel_active(),
            ..self
        }
    }

    pub fn with_observer<O>(self, observer: O) -> ShaderDependency
    where
        O: 'static + Observer<ShaderEvent>,
    {
        assert!(self.is_none());
        let observer: Box<dyn Observer<ShaderEvent>> = Box::new(observer);
        self.with_observer_boxed(observer)
    }

    pub fn with_observer_boxed(self, observer: Box<dyn Observer<ShaderEvent>>) -> ShaderDependency {
        assert!(self.is_none());
        ShaderDependency {
            subscription: SubscriptionRequest::Request(observer),
            ..self
        }
    }

    pub fn with_observer_fn<T>(self, observer: T) -> ShaderDependency
    where
        T: 'static + Send + Sync + Fn(&ShaderEvent) -> ObserveResult,
    {
        assert!(self.is_none());
        let observer: Box<dyn Observer<ShaderEvent>> = Box::new(observer);
        self.with_observer_boxed(observer)
    }

    pub fn with_load_response<D>(
        self,
        load_handler: &AsyncLoadHandler<D>,
        load_token: &LoadToken<D>,
        response: D::LoadResponse,
    ) -> ShaderDependency
    where
        D: OnLoad<LoadHandler = AsyncLoadHandler<D>>,
        D::LoadResponse: Clone,
    {
        let responder = load_handler.responder();
        let load_token = load_token.clone();
        self.with_observer_fn(move |_e| {
            responder.send_response(load_token.clone(), response.clone());
            ObserveResult::KeepObserving
        })
    }

    pub fn is_none(&self) -> bool {
        if let ShaderDependencyIndex::None = self.index {
            true
        } else {
            false
        }
    }

    pub fn key(&self) -> Result<ShaderKey, ShaderDependencyError> {
        if self.id.is_none() {
            log::warn!("Missing shader id");
            Err(ShaderDependencyError)
        } else if self.ty.is_none() {
            log::warn!("Missing shader type");
            Err(ShaderDependencyError)
        } else {
            Ok(self.id.clone().unwrap())
        }
    }

    pub fn request(&mut self, shaders: &mut ShaderStoreRead<'_>) {
        self.index = match mem::replace(&mut self.index, ShaderDependencyIndex::Incomplete) {
            ShaderDependencyIndex::Incomplete => match self.key() {
                Err(err) => ShaderDependencyIndex::Error(err),
                Ok(id) => ShaderDependencyIndex::Pending(shaders.get_or_load(&id)),
            },
            ShaderDependencyIndex::Pending(idx) => {
                let shader = shaders.at(&idx);
                self.subscription.subscribe(&shader.dispatcher);
                match shader.shader() {
                    Err(_) => ShaderDependencyIndex::Error(ShaderDependencyError),
                    Ok(None) => ShaderDependencyIndex::Pending(idx),
                    Ok(Some(st)) => {
                        if st.shader_type != self.ty.unwrap() {
                            ShaderDependencyIndex::Error(ShaderDependencyError)
                        } else {
                            ShaderDependencyIndex::Completed(idx)
                        }
                    }
                }
            }
            ShaderDependencyIndex::None => ShaderDependencyIndex::None,
            ShaderDependencyIndex::Completed(idx) => ShaderDependencyIndex::Completed(idx),
            ShaderDependencyIndex::Error(err) => ShaderDependencyIndex::Error(err),
        }
    }

    pub fn shader_module<'m, 'a: 'm, 's: 'm>(
        &'s mut self,
        shaders: &'a ShaderStoreRead<'m>,
    ) -> Result<Option<&'m CompiledShader>, ShaderDependencyError> {
        match &self.index {
            ShaderDependencyIndex::None | ShaderDependencyIndex::Incomplete | ShaderDependencyIndex::Pending(_) => {
                Ok(None)
            }
            ShaderDependencyIndex::Error(err) => Err(err.clone()),
            ShaderDependencyIndex::Completed(idx) => Ok(shaders.at(idx).shader_module()),
        }
    }
}

impl Default for ShaderDependency {
    fn default() -> Self {
        Self::new()
    }
}
