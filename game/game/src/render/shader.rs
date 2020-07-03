use crate::assets::{AssetError, AssetIO, ShaderType, Url, UrlError};
use crate::render::Context;
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, DataUpdater, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::io;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

/// Helper to manage the state of a shader dependency
pub enum ShaderDependency {
    Pending(ShaderType, ShaderIndex),
    Completed(ShaderIndex),
    Failed,
}

impl ShaderDependency {
    pub fn new<D: Data>(
        shaders: &mut ShaderStoreRead<'_>,
        key: &str,
        shader_type: ShaderType,
        load_context: &LoadContext<'_, D>,
        notify_data: D::LoadResponse,
    ) -> ShaderDependency {
        let id = shaders.get_or_add_blocking(&key.to_owned());
        match shaders.at(&id) {
            Shader::Pending(ref listeners) => {
                listeners.add(load_context, notify_data);
                ShaderDependency::Pending(shader_type, id)
            }
            Shader::Compiled(st, _) => {
                if *st == shader_type {
                    ShaderDependency::Completed(id)
                } else {
                    ShaderDependency::Failed
                }
            }
            Shader::Error => ShaderDependency::Failed,
            Shader::None => unreachable!(),
        }
    }

    pub fn update(self, shaders: &mut ShaderStoreRead<'_>) -> ShaderDependency {
        match self {
            ShaderDependency::Pending(shader_type, id) => match shaders.at(&id) {
                Shader::Pending(_) => ShaderDependency::Pending(shader_type, id),
                Shader::Compiled(st, _) => {
                    if *st == shader_type {
                        ShaderDependency::Completed(id)
                    } else {
                        ShaderDependency::Failed
                    }
                }
                Shader::Error => ShaderDependency::Failed,
                Shader::None => unreachable!(),
            },
            sd => sd,
        }
    }
}

/// Error during shader loading
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

pub enum Shader {
    Pending(LoadListeners),
    Compiled(ShaderType, wgpu::ShaderModule),
    Error,
    None,
}

impl Shader {
    pub fn shadere_module(&self) -> Option<&wgpu::ShaderModule> {
        if let Shader::Compiled(_, ref sh) = self {
            Some(sh)
        } else {
            None
        }
    }

    fn on_update(
        &mut self,
        load_context: LoadContext<'_, Shader>,
        context: &Context,
        load_response: ShaderLoadResponse,
    ) -> Option<String> {
        *self = match (std::mem::replace(self, Shader::None), load_response) {
            (Shader::Pending(listeners), Err(err)) => {
                listeners.notify_all();
                log::warn!("Shader[{:?}] compilation failed: {:?}", load_context, err);
                Shader::Error
            }

            (Shader::Pending(listeners), Ok((ty, spirv))) => {
                listeners.notify_all();
                let shader = context.device().create_shader_module(wgpu::util::make_spirv(&spirv));
                log::debug!("Shader[{:?}] compilation completed", load_context);
                Shader::Compiled(ty, shader)
            }

            _ => unreachable!(),
        };
        None
    }
}

impl Data for Shader {
    type Key = String;
    type LoadRequest = ShaderLoadRequest;
    type LoadResponse = ShaderLoadResponse;
}

impl FromKey for Shader {
    fn from_key(key: &String) -> (Self, Option<String>) {
        (Shader::Pending(LoadListeners::new()), Some(key.to_owned()))
    }
}

pub type ShaderLoadRequest = String;
pub type ShaderLoadResponse = Result<(ShaderType, Vec<u8>), ShaderLoadError>;

pub struct ShaderLoader {
    assetio: Arc<AssetIO>,
}

impl ShaderLoader {
    pub fn new(assetio: Arc<AssetIO>) -> ShaderLoader {
        ShaderLoader { assetio }
    }

    async fn load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Shader>,
        source_id: String,
    ) -> ShaderLoadResponse {
        if cancellation_token.is_canceled() {
            return Err(ShaderLoadError::Canceled);
        }
        let url = Url::parse(&source_id)?;
        let ty = ShaderType::from_str(url.extension())?;
        log::debug!("[{}] Loading shader...", url.as_str());
        let data = self.assetio.download_binary(&url).await?;
        Ok((ty, data))
    }

    async fn try_load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Shader>,
        source_id: String,
    ) -> Option<ShaderLoadResponse> {
        match self.load_from_url(cancellation_token, source_id).await {
            Err(ShaderLoadError::Canceled) => None,
            result => Some(result),
        }
    }
}

impl DataLoader<Shader> for ShaderLoader {
    fn load<'a>(
        &'a mut self,
        source_id: String,
        cancellation_token: CancellationToken<Shader>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<ShaderLoadResponse>>>> {
        Box::pin(self.try_load_from_url(cancellation_token, source_id))
    }
}

impl<'a> DataUpdater<'a, Shader> for (&Context,) {
    fn update<'u>(
        &mut self,
        load_context: LoadContext<'u, Shader>,
        data: &mut Shader,
        load_response: ShaderLoadResponse,
    ) -> Option<ShaderLoadRequest> {
        data.on_update(load_context, self.0, load_response)
    }
}

pub type ShaderStore = Store<Shader>;
pub type ShaderStoreRead<'a> = ReadGuard<'a, Shader>;
pub type ShaderIndex = Index<Shader>;

pub mod systems {
    use super::*;
    use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

    pub fn update_shaders() -> Box<dyn Schedulable> {
        SystemBuilder::new("update_shaders")
            .read_resource::<Context>()
            .write_resource::<ShaderStore>()
            .build(move |_, _, (context, shaders), _| {
                //log::info!("shader");
                let mut shaders = shaders.write();
                let context: &Context = &*context;
                shaders.update(&mut (context,));
                shaders.finalize_requests();
            })
    }

    pub fn gc_shaders() -> Box<dyn Schedulable> {
        SystemBuilder::new("gc_shaders")
            .write_resource::<ShaderStore>()
            .build(move |_, _, shaders, _| {
                let mut shaders = shaders.write();
                shaders.drain_unused();
            })
    }
}
