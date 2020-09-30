struct Inner {
    frame: wgpu::SwapChainTexture,
    descriptor: wgpu::SwapChainDescriptor,
}

#[derive(Default)]
pub struct FrameOutput {
    inner: Option<Inner>,
}

impl FrameOutput {
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
}
