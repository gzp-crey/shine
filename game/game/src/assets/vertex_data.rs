use crate::assets::{Vertex, VertexBufferLayout};

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

    pub fn to_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_with_data(self.get_raw_buffer(), wgpu::BufferUsage::VERTEX)
    }
}
