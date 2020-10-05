use crate::assets::{IndexData, VertexData};
use crate::render::Compile;
use wgpu::util::DeviceExt;

impl Compile<()> for IndexData {
    type Compiled = wgpu::Buffer;

    fn compile(&self, device: &wgpu::Device, _extra: ()) -> Self::Compiled {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: self.get_raw_buffer(),
            usage: wgpu::BufferUsage::INDEX,
        })
    }
}

impl Compile<()> for VertexData {
    type Compiled = wgpu::Buffer;

    fn compile(&self, device: &wgpu::Device, _extra: ()) -> Self::Compiled {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: self.get_raw_buffer(),
            usage: wgpu::BufferUsage::VERTEX,
        })
    }
}
