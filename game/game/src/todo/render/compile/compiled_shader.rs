use crate::{assets::ShaderType, render::Compile};

/// Compiled mesh data ready for rendering
pub struct CompiledShader {
    pub shader_type: ShaderType,
    pub shader: wgpu::ShaderModule,
}

pub struct ShaderCompile<'a> {
    pub shader_type: ShaderType,
    pub data: &'a [u8],
}

impl<'a> Compile for ShaderCompile<'a> {
    type Compiled = CompiledShader;

    fn compile(self, device: &wgpu::Device) -> Self::Compiled {
        let shader = device.create_shader_module(wgpu::util::make_spirv(self.data));
        CompiledShader {
            shader_type: self.shader_type,
            shader,
        }
    }
}
