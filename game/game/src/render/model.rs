use crate::assets::{gltf, AssetError, AssetIO, ModelBuffer, ModelData, Url, UrlError};
use crate::render::{Context, PipelineStore, PipelineStoreRead};
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, DataUpdater, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::pin::Pin;
use std::sync::Arc;

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

impl From<AssetError> for ModelLoadError {
    fn from(err: AssetError) -> ModelLoadError {
        ModelLoadError::Asset(err)
    }
}

pub enum Model {
    Pending(LoadListeners),
    Compiled(ModelBuffer),
    Error,
    None,
}

impl Model {
    pub fn model_buffer(&self) -> Option<&ModelBuffer> {
        if let Model::Compiled(ref model_buffer) = self {
            Some(model_buffer)
        } else {
            None
        }
    }

    fn on_update(
        &mut self,
        load_context: LoadContext<'_, Model>,
        context: &Context,
        _pipelines: &mut PipelineStoreRead<'_>,
        load_response: ModelLoadResponse,
    ) -> Option<String> {
        *self = match (std::mem::replace(self, Model::None), load_response) {
            (Model::Pending(listeners), Err(err)) => {
                log::warn!("Model[{:?}] compilation failed: {:?}", load_context, err);
                listeners.notify_all();
                Model::Error
            }

            (Model::Pending(listeners), Ok(model_data)) => {
                log::debug!("Model[{:?}] compilation completed", load_context);
                listeners.notify_all();
                Model::Compiled(model_data.to_model_buffer(context.device()))
            }

            (Model::Compiled(_), _) => unreachable!(),
            (Model::Error, _) => unreachable!(),
            (Model::None, _) => unreachable!(),
        };
        None
    }
}

impl Data for Model {
    type Key = String;
    type LoadRequest = ModelLoadRequest;
    type LoadResponse = ModelLoadResponse;
}

impl FromKey for Model {
    fn from_key(key: &String) -> (Self, Option<String>) {
        (Model::Pending(LoadListeners::new()), Some(key.to_owned()))
    }
}

pub type ModelLoadRequest = String;
pub type ModelLoadResponse = Result<ModelData, ModelLoadError>;

pub struct ModelLoader {
    base_url: Url,
    assetio: Arc<AssetIO>,
}

impl ModelLoader {
    pub fn new(assetio: Arc<AssetIO>, base_url: Url) -> ModelLoader {
        ModelLoader { base_url, assetio }
    }

    async fn load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Model>,
        source_id: String,
    ) -> ModelLoadResponse {
        if cancellation_token.is_canceled() {
            return Err(ModelLoadError::Canceled);
        }
        let url = self.base_url.join(&source_id)?;
        log::debug!("[{}] Loading model...", url.as_str());
        match url.extension() {
            "gltf" | "glb" => Ok(gltf::load_from_url(&self.assetio, &url).await?),
            ext => Err(ModelLoadError::Asset(AssetError::UnsupportedFormat(ext.to_owned()))),
        }
    }

    async fn try_load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Model>,
        source_id: String,
    ) -> Option<ModelLoadResponse> {
        match self.load_from_url(cancellation_token, source_id).await {
            Err(ModelLoadError::Canceled) => None,
            result => Some(result),
        }
    }
}

impl DataLoader<Model> for ModelLoader {
    fn load<'a>(
        &'a mut self,
        source_id: String,
        cancellation_token: CancellationToken<Model>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<ModelLoadResponse>>>> {
        Box::pin(self.try_load_from_url(cancellation_token, source_id))
    }
}

pub struct ModelUpdater<'a>(&'a Context, &'a PipelineStore);

impl<'a> DataUpdater<'a, Model> for ModelUpdater<'a> {
    fn update<'u>(
        &mut self,
        load_context: LoadContext<'u, Model>,
        data: &mut Model,
        load_response: ModelLoadResponse,
    ) -> Option<ModelLoadRequest> {
        data.on_update(load_context, self.0, &mut self.1.read(), load_response)
    }
}

pub type ModelStore = Store<Model>;
pub type ModelStoreRead<'a> = ReadGuard<'a, Model>;
pub type ModelIndex = Index<Model>;

pub mod systems {
    use super::*;
    use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

    pub fn update_models() -> Box<dyn Schedulable> {
        SystemBuilder::new("update_model")
            .read_resource::<Context>()
            .read_resource::<PipelineStore>()
            .write_resource::<ModelStore>()
            .build(move |_, _, (context, pipelines, models), _| {
                //log::info!("models");
                let mut models = models.write();
                let context: &Context = &*context;
                let pipelines: &PipelineStore = &*pipelines;
                //shaders.drain_unused();
                models.update(&mut ModelUpdater(context, pipelines));
                models.finalize_requests();
            })
    }

    pub fn gc_models() -> Box<dyn Schedulable> {
        SystemBuilder::new("gc_model")
            .write_resource::<ModelStore>()
            .build(move |_, _, models, _| {
                let mut models = models.write();
                models.drain_unused();
            })
    }
}
