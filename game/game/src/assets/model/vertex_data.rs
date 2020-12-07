use crate::assets::{Vertex, VertexBufferLayout};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct VertexData {
    raw: Vec<u8>,
    layout: VertexBufferLayout,
    count: usize,
}

impl VertexData {
    pub fn from_vec<V: Vertex>(data: Vec<V>) -> Self {
        let count = data.len();
        VertexData {
            raw: bytemuck::cast_slice(&data).to_vec(),
            layout: V::buffer_layout(),
            count,
        }
    }

    pub fn get_raw_buffer(&self) -> &[u8] {
        &self.raw
    }

    pub fn get_vertex_layout(&self) -> &VertexBufferLayout {
        &self.layout
    }

    pub fn count(&self) -> usize {
        self.count
    }
}
