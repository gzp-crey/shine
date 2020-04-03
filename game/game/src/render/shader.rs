//use glsl_to_spirv;
use shine_ecs::resources::named::{Data, Store};
use std::mem;
use wgpu;

#[derive(Debug, Clone, Copy)]
pub enum ShaderType {
    Vertex,
    Fragment,
    Compute,
}

pub enum Shader {
    Spirv(ShaderType, Vec<u32>),
    Compiled(ShaderType, wgpu::ShaderModule),
    Error(String),
    None,
}

impl Shader {
    pub fn from_file(file: &str) -> Shader {
        Shader::Error(format!("Missing extension: {}", file))
        /*let ext = match file.rsplit('.', 2).first() {
            Some(ext) => ext,
            None => return wgpu::ShaderModule::Error(format!("Missing extension: {}", file)),
        };*/

        /*let ty = match ext {
            "vs" => glsl_to_spirv::ShaderType::Vertex,
            "fs" => glsl_to_spirv::ShaderType::Fragment,
            "cs" => glsl_to_spirv::ShaderType::Compute,
            _ => return ShaderModule::Error(format!("Unknown shader type: {}", ext)),
        };

        //load from file

        let compiled = match glsl_to_spirv::compile(&code, ty) {
            Ok(compiled) => compiled,
            Err(err) => return ShaderModule::Error(format!("Compile error {}: {:?}", file, err)),
        };

        match wgpu::read_spirv(compiled) {
            Ok(spirv) => ShaderModule::Spirv(ty, spirv),
            Err(err) => ShaderModule::Error(format!("Spirv error {}: {:?}", file, err)),
        }*/
    }

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
    type LoadRequest = String;
    type LoadResponse = ();

    fn from_key(key: String) -> (Shader, Option<Self::LoadRequest>) {
        (Shader::None, Some(key))
    }

    fn update(&mut self, response: Self::LoadResponse) -> Option<Self::LoadRequest> {
        let _ = mem::replace(self, Shader::None);
        None
    }
}
