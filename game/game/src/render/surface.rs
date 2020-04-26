use crate::wgpu;

/// Thread local rendering surface.
pub struct Surface {
    surface: wgpu::Surface,
    size: (u32, u32),
}

impl Surface {
    pub fn new(surface: wgpu::Surface, size: (u32, u32)) -> Surface {
        Surface { surface, size }
    }

    pub fn surface(&self) -> &wgpu::Surface {
        &self.surface
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn set_size(&mut self, size: (u32, u32)) {
        self.size = size;
    }
}
