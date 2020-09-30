use crate::{
    assets::{AssetError, AssetIO, ShaderType, Url},
    render::{Compile, CompiledShader, Context},
};
use shine_ecs::core::{
    observer::{ObserveDispatcher, Subscription},
    store::{
        AsyncLoadHandler, AsyncLoader, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading, ReadGuard,
        Store,
    },
};
use std::{mem, pin::Pin, str::FromStr};

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

impl AssetIO {
    async fn load_shader(
        &self,
        load_token: LoadToken<Shader>,
        source_id: String,
    ) -> Result<(ShaderType, Vec<u8>), ShaderLoadError> {
        log::debug!("[{:?}] Loading shader...", load_token);

        let url = Url::parse(&source_id).map_err(|err| AssetError::InvalidUrl(err))?;
        let ty = ShaderType::from_str(url.extension())?;
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

#[derive(Debug, Clone)]
pub struct ShaderDependencyError;

enum ShaderDependencyIndex {
    None,
    Incomplete,
    Pending(ShaderIndex, Option<Subscription<ShaderEvent>>),
    Completed(ShaderIndex, Option<Subscription<ShaderEvent>>),
    Error(ShaderDependencyError),
}

pub struct ShaderDependency {
    ty: Option<ShaderType>,
    id: Option<String>,
    index: ShaderDependencyIndex,
}

impl Default for ShaderDependency {
    fn default() -> Self {
        Self {
            ty: None,
            id: None,
            index: ShaderDependencyIndex::Incomplete,
        }
    }
}

impl ShaderDependency {
    pub fn none() -> ShaderDependency {
        ShaderDependency {
            ty: None,
            id: None,
            index: ShaderDependencyIndex::None,
        }
    }

    pub fn with_type(self, ty: ShaderType) -> ShaderDependency {
        assert!(!self.is_none());
        ShaderDependency {
            ty: Some(ty),
            index: ShaderDependencyIndex::Incomplete,
            ..self
        }
    }

    pub fn or_type(&mut self, ty: ShaderType) -> &mut ShaderDependency {
        assert!(!self.is_none());
        if self.ty.is_none() {
            self.ty = Some(ty);
            self.index = ShaderDependencyIndex::Incomplete;
        }
        self
    }

    pub fn with_id<S: ToString>(self, id: S) -> ShaderDependency {
        assert!(!self.is_none());
        ShaderDependency {
            id: Some(id.to_string()),
            index: ShaderDependencyIndex::Incomplete,
            ..self
        }
    }

    pub fn or_id<S: ToString>(&mut self, id: S) -> &mut ShaderDependency {
        assert!(!self.is_none());
        if self.id.is_none() {
            self.id = Some(id.to_string());
            self.index = ShaderDependencyIndex::Incomplete;
        }
        self
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

    pub fn compiled_shader<'c, 'r: 'c, 's: 'c>(
        &'s mut self,
        shaders: &'r ShaderStoreRead<'c>,
    ) -> Result<Option<&'c CompiledShader>, ShaderDependencyError> {
        match &self.index {
            ShaderDependencyIndex::None | ShaderDependencyIndex::Incomplete => Ok(None),
            ShaderDependencyIndex::Pending(_, _) => Ok(None),
            ShaderDependencyIndex::Error(err) => Err(err.clone()),
            ShaderDependencyIndex::Completed(idx, _) => Ok(shaders.at(idx).shader_module()),
        }
    }

    pub fn request_with<'c, 'r: 'c, 's: 'c, S>(
        &'s mut self,
        shaders: &'r ShaderStoreRead<'c>,
        subscription: S,
    ) -> Result<Option<&'c CompiledShader>, ShaderDependencyError>
    where
        S: FnOnce(&Shader) -> Option<Subscription<ShaderEvent>>,
    {
        self.index = match mem::replace(&mut self.index, ShaderDependencyIndex::Incomplete) {
            ShaderDependencyIndex::Incomplete => match self.key() {
                Err(err) => ShaderDependencyIndex::Error(err),
                Ok(id) => {
                    let idx = shaders.get_or_load(&id);
                    let sub = subscription(&shaders.at(&idx));
                    ShaderDependencyIndex::Pending(idx, sub)
                }
            },
            ShaderDependencyIndex::Pending(idx, sub) => {
                let shader = shaders.at(&idx);
                match shader.shader() {
                    Err(_) => ShaderDependencyIndex::Error(ShaderDependencyError),
                    Ok(None) => ShaderDependencyIndex::Pending(idx, sub),
                    Ok(Some(st)) => {
                        if st.shader_type != self.ty.unwrap() {
                            ShaderDependencyIndex::Error(ShaderDependencyError)
                        } else {
                            ShaderDependencyIndex::Completed(idx, sub)
                        }
                    }
                }
            }
            ShaderDependencyIndex::None => ShaderDependencyIndex::None,
            ShaderDependencyIndex::Completed(idx, sub) => ShaderDependencyIndex::Completed(idx, sub),
            ShaderDependencyIndex::Error(err) => ShaderDependencyIndex::Error(err),
        };

        self.compiled_shader(shaders)
    }

    pub fn request<'c, 'r: 'c, 's: 'c>(
        &'s mut self,
        shaders: &'r ShaderStoreRead<'c>,
    ) -> Result<Option<&'c CompiledShader>, ShaderDependencyError> {
        self.request_with(shaders, |_| None)
    }
}
