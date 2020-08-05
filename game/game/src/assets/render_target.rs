use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RenderTargetSize {
    /// Size matching the frame output
    Matching,
    /// Size propotional to the render target
    Propotional(f32, f32),
    /// Fixed sized
    Fixed(u32, u32),
}

/// Render target descriptor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderTargetDescriptor {
    pub format: wgpu::TextureFormat,
    pub size: RenderTargetSize,
}

impl RenderTargetDescriptor {
    pub fn get_target_size(&self, frame_size: (u32, u32)) -> (u32, u32) {
        match &self.size {
            RenderTargetSize::Matching => frame_size,
            RenderTargetSize::Fixed(w, h) => (*w, *h),
            RenderTargetSize::Propotional(sw, sh) => {
                let w = ((frame_size.0 as f32) * sw).clamp(4., 65536.) as u32;
                let h = ((frame_size.1 as f32) * sh).clamp(4., 65536.) as u32;
                (w, h)
            }
        }
    }
}
