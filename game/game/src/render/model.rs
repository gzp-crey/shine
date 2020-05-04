use crate::utils::url::Url;
use crate::{
    render::{gltf, Context, ModelBuffer, ModelData, PipelineStore, PipelineStoreRead},
    GameError,
};
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, DataUpdater, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::pin::Pin;

pub enum Model {
    Pending(LoadListeners),
    Compiled(ModelBuffer),
    Error(String),
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
            (Model::Pending(listeners), ModelLoadResponse::Error(err)) => {
                log::debug!("Model compilation failed [{:?}]: {:?}", load_context, err);
                listeners.notify_all();
                Model::Error(err)
            }

            (Model::Pending(listeners), ModelLoadResponse::ModelData(model_data)) => {
                log::debug!("Model compilation completed for [{:?}]", load_context);
                listeners.notify_all();
                Model::Compiled(model_data.to_model_buffer(context.device()))
            }

            (Model::Compiled(_), _) => unreachable!(),
            (Model::Error(_), _) => unreachable!(),
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

pub enum ModelLoadResponse {
    ModelData(ModelData),
    Error(String),
}

pub struct ModelLoader {
    base_url: Url,
}

impl ModelLoader {
    pub fn new(base_url: &str) -> Result<ModelLoader, GameError> {
        let base_url = Url::parse(base_url)
            .map_err(|err| GameError::Config(format!("Failed to parse base url for model: {:?}", err)))?;

        Ok(ModelLoader { base_url })
    }

    async fn load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Model>,
        source_id: String,
    ) -> Option<ModelLoadResponse> {
        if cancellation_token.is_canceled() {
            return None;
        }

        let url = match self.base_url.join(&source_id) {
            Err(err) => {
                let err = format!("Invalid model url ({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(ModelLoadResponse::Error(err));
            }
            Ok(url) => url,
        };

        log::debug!("Model loading: [{}]", url.as_str());
        match url.extension() {
            "gltf" | "glb" => match gltf::load_from_url(&url).await {
                Err(err) => {
                    let err = format!("Failed to load model from ({}): {:?}", source_id, err);
                    log::warn!("{}", err);
                    Some(ModelLoadResponse::Error(err))
                }
                Ok(data) => Some(ModelLoadResponse::ModelData(data)),
            },
            ext => {
                let err = format!("Unknown model type ({})", ext);
                log::warn!("{}", err);
                Some(ModelLoadResponse::Error(err))
            }
        }
    }
}

impl DataLoader<Model> for ModelLoader {
    fn load<'a>(
        &'a mut self,
        source_id: String,
        cancellation_token: CancellationToken<Model>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<ModelLoadResponse>>>> {
        Box::pin(self.load_from_url(cancellation_token, source_id))
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
}
