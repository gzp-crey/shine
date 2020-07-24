use crate::{assets::ShaderType, render::Compile};

/// Compiled mesh data ready for rendering
pub struct CompiledShader {
    pub shader_type: ShaderType,
    pub shader: wgpu::ShaderModule,
}

impl Compile<()> for (ShaderType, &[u8]) {
    type Compiled = CompiledShader;

    fn compile(&self, device: &wgpu::Device, _extra: ()) -> Self::Compiled {
        let shader = device.create_shader_module(wgpu::util::make_spirv(self.1));
        CompiledShader {
            shader_type: self.0,
            shader,
        }
    }
}
