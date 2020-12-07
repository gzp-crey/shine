use crate::{
    assets::{CookedShader, ShaderType},
    render::Compile,
};

/// Compiled shader, ready to use
pub struct CompiledShader {
    pub shader_type: ShaderType,
    pub shader: wgpu::ShaderModule,
}

impl<'a> Compile for &'a CookedShader {
    type Output = CompiledShader;

    fn compile(self, device: &wgpu::Device) -> Self::Output {
        let shader = device.create_shader_module(wgpu::util::make_spirv(&self.binary));
        CompiledShader {
            shader_type: self.shader_type,
            shader,
        }
    }
}
