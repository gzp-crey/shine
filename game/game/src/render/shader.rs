use crate::utils::url::Url;
use crate::{render::Context, utils, wgpu, GameError};
use futures::future::FutureExt;
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::pin::Pin;

#[derive(Debug, Clone, Copy)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

pub enum Shader {
    Pending(LoadListeners),
    Compiled(ShaderType, wgpu::ShaderModule),
    Error(String),
}

impl Shader {
    pub fn is_ready<'a, D: Data>(&self, listener: &LoadContext<'a, D>, data: D::LoadResponse) -> Result<bool, String> {
        match *self {
            Shader::Pending(ref listeners) => {
                listeners.add(listener, data);
                Ok(false)
            }
            Shader::Compiled(..) => Ok(true),
            Shader::Error(ref err) => Err(err.clone()),
        }
    }
}

pub enum ShaderLoadResult {
    Spirv(ShaderType, Vec<u32>),
    Error(String),
}

impl Data for Shader {
    type Key = String;
    type LoadRequest = String;
    type LoadResponse = ShaderLoadResult;
    type UpdateContext = Context;

    fn on_load(
        &mut self,
        load_context: LoadContext<Shader>,
        context: &Context,
        load_response: ShaderLoadResult,
    ) -> Option<String> {
        match load_response {
            ShaderLoadResult::Error(err) => {
                *self = Shader::Error(err);
            }

            ShaderLoadResult::Spirv(ty, spirv) => {
                log::info!("Compile shader {:?}", load_context.key());
                *self = Shader::Compiled(ty, context.device().create_shader_module(&spirv));
            }
        }
        None
    }
}

impl FromKey for Shader {
    fn from_key(key: &String) -> (Self, Option<String>) {
        (Shader::Pending(LoadListeners::new()), Some(key.to_owned()))
    }
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
    ) -> Option<ShaderLoadResult> {
        if cancellation_token.is_canceled() {
            return None;
        }

        let url = match self.base_url.join(&source_id) {
            Err(err) => {
                let err = format!("Invalid shader url ({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(ShaderLoadResult::Error(err));
            }
            Ok(url) => url,
        };

        log::info!("loading: {}", url.as_str());
        let ty = match url.extension() {
            "fs_spv" => ShaderType::Fragment,
            "vs_spv" => ShaderType::Vertex,
            "cs_spv" => ShaderType::Compute,
            ext => {
                let err = format!("Unknown shader type ({})", ext);
                log::warn!("{}", err);
                return Some(ShaderLoadResult::Error(err));
            }
        };

        let data = match utils::assets::download_binary(&url).await {
            Err(err) => {
                let err = format!("Failed to get shader({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(ShaderLoadResult::Error(err));
            }
            Ok(data) => data,
        };

        use std::io::Cursor;
        let spirv = match wgpu::read_spirv(Cursor::new(&data[..])) {
            Err(err) => {
                let err = format!("Failed to read spirv ({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(ShaderLoadResult::Error(err));
            }
            Ok(spirv) => spirv,
        };

        Some(ShaderLoadResult::Spirv(ty, spirv))
    }
}

impl DataLoader<Shader> for ShaderLoader {
    fn load<'a>(
        &'a mut self,
        source_id: String,
        cancellation_token: CancellationToken<Shader>,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<ShaderLoadResult>> + Send + 'a>> {
        self.load_from_url(cancellation_token, source_id).boxed()
    }
}

pub type ShaderStore = Store<Shader>;
pub type ReadShaderStore<'a> = ReadGuard<'a, Shader>;
pub type ShaderIndex = Index<Shader>;
