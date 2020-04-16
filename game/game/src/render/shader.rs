//use glsl_to_spirv;
use crate::wgpu;
use futures::future::FutureExt;
use shine_ecs::core::store::{Data, DataLoader, FromKey, LoadContext, Store};
use std::mem;
use std::pin::Pin;

#[derive(Debug, Clone, Copy)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

pub enum Shader {
    Source(String),
    Spirv(ShaderType, Vec<u32>),
    Compiled(ShaderType, wgpu::ShaderModule),
    Error(String),
    None,
}

impl Shader {
    pub fn compile(&mut self, device: &wgpu::Device) {
        let module = match self {
            Shader::Spirv(ty, spirv) => Some(Shader::Compiled(*ty, device.create_shader_module(&spirv))),
            _ => None,
        };

        if let Some(module) = module {
            let _ = mem::replace(self, module);
        }
    }
}

impl Data for Shader {
    type Key = String;
    type LoadRequest = Self;
    type LoadResponse = Self;

    fn on_load(&mut self, load_response: Option<Self::LoadResponse>) -> Option<Self::LoadRequest> {
        match load_response {
            Some(response) => {
                let _ = mem::replace(self, Shader::None);
                None
            }
            None => {
                if let Shader::Source(ref src) = self {
                    Some(Shader::Source(src.to_owned()))
                } else {
                    None
                }
            }
        }
    }
}

impl FromKey for Shader {
    fn from_key(key: &Self::Key) -> Self {
        Shader::Source(key.clone())
    }
}

async fn load_spirv_from_file(file: String) -> Option<Shader> {
    unimplemented!()
}

pub struct ShaderLoader;

impl DataLoader<Shader> for ShaderLoader {
    fn load(
        &mut self,
        request: Shader,
        context: &mut LoadContext<Shader>,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<Shader>> + Send>> {
        match request {
            Shader::Source(file) => load_spirv_from_file(file).boxed(),
            _ => unimplemented!(),
        }
    }
}

pub type ShaderStore = Store<Shader>;
