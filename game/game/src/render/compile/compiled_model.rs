use crate::assets::{CookedModel, MeshData, MODEL_MAX_LOD_COUNT};
use crate::render::Compile;

/// Compiled mesh data ready for rendering
pub struct CompiledMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: Option<wgpu::Buffer>,
    pub lod: [(usize, usize); MODEL_MAX_LOD_COUNT],
}

impl<'a> Compile for &'a MeshData {
    type Output = CompiledMesh;

    fn compile(self, device: &wgpu::Device) -> Self::Output {
        CompiledMesh {
            vertex_buffer: self.vertices.compile(device),
            index_buffer: self.indices.as_ref().map(|indices| indices.compile(device)),
            lod: self.lod,
        }
    }
}

/// Compiled model ready for rendering
pub struct CompiledModel {
    pub meshes: Vec<CompiledMesh>,
}

impl<'a> Compile for &'a CookedModel {
    type Output = CompiledModel;

    fn compile(self, device: &wgpu::Device) -> Self::Output {
        CompiledModel {
            meshes: self.meshes.iter().map(|mesh| mesh.compile(device)).collect(),
        }
    }
}
