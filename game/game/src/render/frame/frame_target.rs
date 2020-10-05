use shine_ecs::resources::{Res, ResMut};

struct Inner {
    frame: wgpu::SwapChainTexture,
    descriptor: wgpu::SwapChainDescriptor,
}

#[derive(Default)]
pub struct FrameTarget {
    inner: Option<Inner>,
}

impl FrameTarget {
    pub fn set(&mut self, frame: wgpu::SwapChainTexture, descriptor: wgpu::SwapChainDescriptor) {
        self.inner = Some(Inner { frame, descriptor });
    }

    pub fn present(&mut self) {
        self.inner = None;
    }

    pub fn size(&self) -> (u32, u32) {
        self.inner
            .as_ref()
            .map(|x| (x.descriptor.width, x.descriptor.height))
            .unwrap_or((0, 0))
    }

    pub fn descriptor(&self) -> Option<&wgpu::SwapChainDescriptor> {
        self.inner.as_ref().map(|x| &x.descriptor)
    }
}

pub type FrameTargetRes<'a> = Res<'a, FrameTarget>;
pub type FrameTargetResMut<'a> = ResMut<'a, FrameTarget>;
