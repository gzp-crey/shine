use crate::assets::{IndexData, VertexData};
use crate::render::Compile;
use wgpu::util::DeviceExt;

impl<'a> Compile for &'a IndexData {
    type Compiled = wgpu::Buffer;

    fn compile(self, device: &wgpu::Device) -> Self::Compiled {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: self.get_raw_buffer(),
            usage: wgpu::BufferUsage::INDEX,
        })
    }
}

impl<'a> Compile for &'a VertexData {
    type Compiled = wgpu::Buffer;

    fn compile(self, device: &wgpu::Device) -> Self::Compiled {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: self.get_raw_buffer(),
            usage: wgpu::BufferUsage::VERTEX,
        })
    }
}
