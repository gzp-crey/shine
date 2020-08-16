use crate::assets::IndexData;
use crate::assets::VertexData;
use wgpu::util::DeviceExt;

pub trait Compile<E> {
    type Compiled;

    fn compile(&self, device: &wgpu::Device, extra: E) -> Self::Compiled;
}

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
