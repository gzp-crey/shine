/// Trait for render resource compilation. Once a resource is compiled, the
/// source can be thrown away.
pub trait Compile {
    type Output;

    fn compile(self, device: &wgpu::Device) -> Self::Output;
}

mod compiled_buffers;
pub use self::compiled_buffers::*;
mod compiled_shader;
pub use self::compiled_shader::*;
mod compiled_texture;
pub use self::compiled_texture::*;
//mod compiled_texture_target;
//pub use self::compiled_texture_target::*;
mod compiled_pipeline;
pub use self::compiled_pipeline::*;
mod compiled_model;
pub use self::compiled_model::*;
