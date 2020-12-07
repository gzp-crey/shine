use crate::assets::{IndexData, VertexData};
use serde::{Deserialize, Serialize};

pub const MODEL_MAX_LOD_COUNT: usize = 4;

#[derive(Serialize, Deserialize)]
pub struct MeshData {
    pub vertices: VertexData,
    pub indices: Option<IndexData>,
    /// (start, count) sections for each lod
    pub lod: [(usize, usize); MODEL_MAX_LOD_COUNT],
}

impl MeshData {
    pub fn with_vertices(vertices: VertexData) -> MeshData {
        let cnt = vertices.count();
        MeshData {
            vertices,
            indices: None,
            lod: [(0, cnt); MODEL_MAX_LOD_COUNT],
        }
    }

    pub fn with_vertices_and_indices(vertices: VertexData, indices: IndexData) -> MeshData {
        let cnt = indices.len();
        MeshData {
            vertices,
            indices: Some(indices),
            lod: [(0, cnt); MODEL_MAX_LOD_COUNT],
        }
    }
}
