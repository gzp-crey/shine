use crate::utils::url::Url;
use crate::{render::Context, utils, wgpu, GameError};
use futures::future::FutureExt;
use shine_ecs::core::store::{Data, DataLoader, FromKey, LoadContext, Store};
use std::pin::Pin;

#[derive(Debug, Clone, Copy)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

pub enum Shader {
    None,
    Compiled(ShaderType, wgpu::ShaderModule),
    Error(String),
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

    fn on_load(&mut self, key: Option<&String>, context: &Context, load_response: ShaderLoadResult) -> Option<String> {
        match load_response {
            ShaderLoadResult::Error(err) => {
                *self = Shader::Error(err);
            }

            ShaderLoadResult::Spirv(ty, spirv) => {
                log::info!("Compile shader {:?}", key);
                *self = Shader::Compiled(ty, context.device().create_shader_module(&spirv));
            }
        }
        None
    }
}

impl FromKey for Shader {
    fn from_key(key: &String) -> (Self, Option<String>) {
        (Shader::None, Some(key.to_owned()))
    }
}

pub struct ShaderLoader {
    base_url: Url,
}

impl ShaderLoader {
    pub fn new(base_url: &str) -> Result<ShaderLoader, GameError> {
        let base_url = Url::parse(base_url)
            .map_err(|err| GameError::Config(format!("Failes to parse base url for shaders: {:?}", err)))?;

        Ok(ShaderLoader { base_url })
    }

    async fn load_spirv_from_url(
        &mut self,
        context: LoadContext<Shader>,
        shader_file: String,
    ) -> Option<ShaderLoadResult> {
        if context.is_canceled() {
            return None;
        }

        let url = match self.base_url.join(&shader_file) {
            Err(err) => {
                let err = format!("Invalid shader url ({}): {:?}", shader_file, err);
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
                let err = format!("Failed to get shader({}): {:?}", shader_file, err);
                log::warn!("{}", err);
                return Some(ShaderLoadResult::Error(err));
            }
            Ok(data) => data,
        };

        use std::io::Cursor;
        let spirv = match wgpu::read_spirv(Cursor::new(&data[..])) {
            Err(err) => {
                let err = format!("Failed to read spirv ({}): {:?}", shader_file, err);
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
        request: String,
        context: LoadContext<Shader>,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<ShaderLoadResult>> + Send + 'a>> {
        self.load_spirv_from_url(context, request).boxed()
    }
}

pub type ShaderStore = Store<Shader>;
