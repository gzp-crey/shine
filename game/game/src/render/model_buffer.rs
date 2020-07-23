use crate::assets::{MeshData, ModelData, MODEL_MAX_LOD_COUNT};
use crate::render::Compile;

/// Compiled mesh data ready for rendering
pub struct MeshBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: Option<wgpu::Buffer>,
    pub lod: [(usize, usize); MODEL_MAX_LOD_COUNT],
}

impl Compile<()> for MeshData {
    type Compiled = MeshBuffer;

    fn compile(&self, device: &wgpu::Device, _extra: ()) -> Self::Compiled {
        MeshBuffer {
            vertex_buffer: self.vertices.compile(device, ()),
            index_buffer: self.indices.as_ref().map(|indices| indices.compile(device, ())),
            lod: self.lod,
        }
    }
}

/// Compiled model ready for rendering
pub struct ModelBuffer {
    pub meshes: Vec<MeshBuffer>,
}

impl Compile<()> for ModelData {
    type Compiled = ModelBuffer;

    fn compile(&self, device: &wgpu::Device, _extra: ()) -> Self::Compiled {
        ModelBuffer {
            meshes: self.meshes.iter().map(|mesh| mesh.compile(device, ())).collect(),
        }
    }
}
