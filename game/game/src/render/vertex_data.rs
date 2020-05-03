use crate::render::{Vertex, VertexBufferLayout, VertexP2C3};

pub enum VertexData {
    Pos2Col3(Vec<VertexP2C3>),
}

impl VertexData {
    pub fn from_p2c3(data: Vec<VertexP2C3>) -> Self {
        VertexData::Pos2Col3(data)
    }

    pub fn len(&self) -> usize {
        match *self {
            VertexData::Pos2Col3(ref data) => data.len(),
        }
    }

    pub fn get_raw_buffer(&self) -> &[u8] {
        match *self {
            VertexData::Pos2Col3(ref data) => bytemuck::cast_slice(data),
        }
    }

    pub fn get_vertex_layout(&self) -> VertexBufferLayout {
        match *self {
            VertexData::Pos2Col3(_) => VertexP2C3::buffer_layout(),
        }
    }

    pub fn to_vertex_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_with_data(self.get_raw_buffer(), wgpu::BufferUsage::VERTEX)
    }
}
