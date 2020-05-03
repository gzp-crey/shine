use crate::utils::url::Url;
use crate::{render::Context, utils, wgpu, GameError};
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, DataUpdater, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::pin::Pin;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

/// Helper to manage the state of a shader dependency
pub enum ShaderDependency {
    Pending(ShaderType, ShaderIndex),
    Completed(ShaderIndex),
    Failed(String),
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
                    ShaderDependency::Failed(format!(
                        "Shader type missmatch, expected: {:?}, found: {:?}",
                        shader_type, st
                    ))
                }
            }
            Shader::Error(ref err) => ShaderDependency::Failed(err.clone()),
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
                        ShaderDependency::Failed(format!(
                            "Shader type missmatch, expected: {:?}, found: {:?}",
                            shader_type, st
                        ))
                    }
                }
                Shader::Error(ref err) => ShaderDependency::Failed(err.to_owned()),
                Shader::None => unreachable!(),
            },
            sd => sd,
        }
    }
}

pub enum Shader {
    Pending(LoadListeners),
    Compiled(ShaderType, wgpu::ShaderModule),
    Error(String),
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

    fn on_load(
        &mut self,
        load_context: LoadContext<'_, Shader>,
        context: &Context,
        load_response: ShaderLoadResponse,
    ) -> Option<String> {
        *self = match (std::mem::replace(self, Shader::None), load_response) {
            (Shader::Pending(listeners), ShaderLoadResponse::Error(err)) => {
                log::debug!("Shader compilation failed [{:?}]: {:?}", load_context, err);
                listeners.notify_all();
                Shader::Error(err)
            }

            (Shader::Pending(listeners), ShaderLoadResponse::Spirv(ty, spirv)) => {
                log::debug!("Shader compilation completed for [{:?}]", load_context);
                listeners.notify_all();
                Shader::Compiled(ty, context.device().create_shader_module(&spirv))
            }

            (Shader::Compiled(_, _), _) => unreachable!(),
            (Shader::Error(_), _) => unreachable!(),
            (Shader::None, _) => unreachable!(),
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

pub enum ShaderLoadResponse {
    Spirv(ShaderType, Vec<u32>),
    Error(String),
}

pub struct ShaderLoader {
    base_url: Url,
}

impl ShaderLoader {
    pub fn new(base_url: &str) -> Result<ShaderLoader, GameError> {
        let base_url = Url::parse(base_url)
            .map_err(|err| GameError::Config(format!("Failed to parse base url for shaders: {:?}", err)))?;

        Ok(ShaderLoader { base_url })
    }

    async fn load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Shader>,
        source_id: String,
    ) -> Option<ShaderLoadResponse> {
        if cancellation_token.is_canceled() {
            return None;
        }

        let url = match self.base_url.join(&source_id) {
            Err(err) => {
                let err = format!("Invalid shader url ({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(ShaderLoadResponse::Error(err));
            }
            Ok(url) => url,
        };

        log::debug!("Shader loading: [{}]", url.as_str());
        let ty = match url.extension() {
            "fs_spv" => ShaderType::Fragment,
            "vs_spv" => ShaderType::Vertex,
            "cs_spv" => ShaderType::Compute,
            ext => {
                let err = format!("Unknown shader type ({})", ext);
                log::warn!("{}", err);
                return Some(ShaderLoadResponse::Error(err));
            }
        };

        let data = match utils::assets::download_binary(&url).await {
            Err(err) => {
                let err = format!("Failed to get shader({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(ShaderLoadResponse::Error(err));
            }
            Ok(data) => data,
        };

        use std::io::Cursor;
        let spirv = match wgpu::read_spirv(Cursor::new(&data[..])) {
            Err(err) => {
                let err = format!("Failed to read spirv ({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(ShaderLoadResponse::Error(err));
            }
            Ok(spirv) => spirv,
        };

        Some(ShaderLoadResponse::Spirv(ty, spirv))
    }
}

impl DataLoader<Shader> for ShaderLoader {
    fn load<'a>(
        &'a mut self,
        source_id: String,
        cancellation_token: CancellationToken<Shader>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<ShaderLoadResponse>>>> {
        Box::pin(self.load_from_url(cancellation_token, source_id))
    }
}

impl<'a> DataUpdater<'a, Shader> for (&Context,) {
    fn update<'u>(
        &mut self,
        load_context: LoadContext<'u, Shader>,
        data: &mut Shader,
        load_response: ShaderLoadResponse,
    ) -> Option<ShaderLoadRequest> {
        data.on_load(load_context, self.0, load_response)
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
                //shaders.drain_unused();
                let context: &Context = &*context;
                shaders.update(&mut (context,));
                shaders.finalize_requests();
            })
    }
}
