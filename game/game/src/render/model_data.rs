use crate::render::{IndexData, VertexData};

pub const MAX_LOD_COUNT: usize = 4;

pub struct MeshBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: Option<wgpu::Buffer>,
    pub lod: [(usize, usize); MAX_LOD_COUNT],
}

pub struct MeshData {
    pub vertices: VertexData,
    pub indices: Option<IndexData>,
    /// (start, count) sections for each lod
    pub lod: [(usize, usize); MAX_LOD_COUNT],
}

impl MeshData {
    pub fn with_vertices(vertices: VertexData) -> MeshData {
        let cnt = vertices.len();
        MeshData {
            vertices,
            indices: None,
            lod: [(0, cnt); MAX_LOD_COUNT],
        }
    }

    pub fn with_vertices_and_indices(vertices: VertexData, indices: IndexData) -> MeshData {
        let cnt = indices.len();
        MeshData {
            vertices,
            indices: Some(indices),
            lod: [(0, cnt); MAX_LOD_COUNT],
        }
    }

    pub fn to_mesh_buffer(&self, device: &wgpu::Device) -> MeshBuffer {
        MeshBuffer {
            vertex_buffer: self.vertices.to_vertex_buffer(device),
            index_buffer: self.indices.as_ref().map(|indices| indices.to_index_buffer(device)),
            lod: self.lod.clone(),
        }
    }
}

pub struct ModelBuffer {
    pub meshes: Vec<MeshBuffer>,
}

pub struct ModelData {
    pub meshes: Vec<MeshData>,
}

impl ModelData {
    pub fn new() -> ModelData {
        ModelData { meshes: Vec::new() }
    }

    pub fn to_model_buffer(&self, device: &wgpu::Device) -> ModelBuffer {
        ModelBuffer {
            meshes: self.meshes.iter().map(|mesh| mesh.to_mesh_buffer(device)).collect(),
        }
    }
}
