use crate::assets::{gltf, AssetError, AssetIO, ModelData, Url, UrlError};
use crate::render::{Compile, CompiledModel, Context};
use shine_ecs::core::store::{
    AsyncLoadHandler, AsyncLoader, Data, FromKey, Index, LoadCanceled, LoadToken, OnLoad, OnLoading, ReadGuard, Store,
};
use std::pin::Pin;

/// Unique key for a model
pub type ModelKey = String;

#[derive(Debug, Clone)]
pub struct ModelError;

pub struct Model {
    id: String,
    model: Result<Option<CompiledModel>, ModelError>,
}

impl Model {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn model(&self) -> Result<Option<&CompiledModel>, ModelError> {
        match &self.model {
            Err(_) => Err(ModelError),
            Ok(None) => Ok(None),
            Ok(Some(model)) => Ok(Some(model)),
        }
    }

    pub fn model_module(&self) -> Option<&CompiledModel> {
        if let Ok(Some(model)) = &self.model {
            Some(model)
        } else {
            None
        }
    }
}

impl Data for Model {
    type Key = ModelKey;
}

impl FromKey for Model {
    fn from_key(key: &ModelKey) -> Self {
        Model {
            id: key.to_owned(),
            model: Ok(None),
        }
    }
}

impl<'l> OnLoading<'l> for Model {
    type LoadingContext = (&'l Context,);
}

impl OnLoad for Model {
    type LoadRequest = ModelLoadRequest;
    type LoadResponse = ModelLoadResponse;
    type LoadHandler = AsyncLoadHandler<Self>;

    fn on_load_request(&mut self, load_handler: &mut Self::LoadHandler, load_token: LoadToken<Self>) {
        load_handler.request(load_token, ModelLoadRequest(self.id.clone()));
    }

    fn on_load_response<'l>(
        &mut self,
        _load_handler: &mut Self::LoadHandler,
        load_context: &mut (&'l Context,),
        load_token: LoadToken<Self>,
        load_response: ModelLoadResponse,
    ) {
        let (context,) = (load_context.0,);
        match load_response.0 {
            Err(err) => {
                self.model = Err(ModelError);
                //self.listeners.notify_all();
                log::warn!("[{:?}] Model compilation failed: {:?}", load_token, err);
            }

            Ok(model_data) => {
                self.model = Ok(Some(model_data.compile(context.device())));
                //self.listeners.notify_all();
                log::debug!("[{:?}] Model compilation completed", load_token);
            }
        };
    }
}

pub struct ModelLoadRequest(ModelKey);
pub struct ModelLoadResponse(Result<ModelData, ModelLoadError>);

/// Error during model loading.
#[derive(Debug)]
pub enum ModelLoadError {
    Asset(AssetError),
    Canceled,
}

impl From<UrlError> for ModelLoadError {
    fn from(err: UrlError) -> ModelLoadError {
        ModelLoadError::Asset(AssetError::InvalidUrl(err))
    }
}

impl From<LoadCanceled> for ModelLoadError {
    fn from(_err: LoadCanceled) -> ModelLoadError {
        ModelLoadError::Canceled
    }
}

impl From<AssetError> for ModelLoadError {
    fn from(err: AssetError) -> ModelLoadError {
        ModelLoadError::Asset(err)
    }
}

impl AssetIO {
    async fn load_model(&self, load_token: LoadToken<Model>, source_id: String) -> Result<ModelData, ModelLoadError> {
        let url = Url::parse(&source_id)?;
        log::debug!("[{:?}] Loading model...", load_token);
        match url.extension() {
            "gltf" | "glb" => Ok(gltf::load_from_url(&self, &url).await?),
            ext => Err(ModelLoadError::Asset(AssetError::UnsupportedFormat(ext.to_owned()))),
        }
    }
}

impl AsyncLoader<Model> for AssetIO {
    fn load<'l>(
        &'l mut self,
        load_token: LoadToken<Model>,
        request: ModelLoadRequest,
    ) -> Pin<Box<dyn 'l + std::future::Future<Output = Option<ModelLoadResponse>>>> {
        Box::pin(async move {
            match self.load_model(load_token, request.0).await {
                Err(ModelLoadError::Canceled) => None,
                result => Some(ModelLoadResponse(result)),
            }
        })
    }
}

pub type ModelStore = Store<Model, AsyncLoadHandler<Model>>;
pub type ModelStoreRead<'a> = ReadGuard<'a, Model, AsyncLoadHandler<Model>>;
pub type ModelIndex = Index<Model>;
