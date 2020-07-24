use crate::assets::{IndexData,VertexData};
use crate::render::Compile;

impl Compile<()> for IndexData {
    type Compiled = wgpu::Buffer;

    fn compile(&self, device: &wgpu::Device, _extra: ()) -> Self::Compiled {
        device.create_buffer_with_data(self.get_raw_buffer(), wgpu::BufferUsage::INDEX)
    }
}

impl Compile<()> for VertexData {
    type Compiled = wgpu::Buffer;

    fn compile(&self, device: &wgpu::Device, _extra: ()) -> Self::Compiled {
        device.create_buffer_with_data(self.get_raw_buffer(), wgpu::BufferUsage::VERTEX)
    }
}
