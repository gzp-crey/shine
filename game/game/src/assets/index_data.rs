pub struct IndexData(Vec<u16>);

impl IndexData {
    pub fn new(data: Vec<u16>) -> IndexData {
        IndexData(data)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get_raw_buffer(&self) -> &[u8] {
        bytemuck::cast_slice(&self.0)
    }

    pub fn to_index_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_with_data(self.get_raw_buffer(), wgpu::BufferUsage::INDEX)
    }
}
