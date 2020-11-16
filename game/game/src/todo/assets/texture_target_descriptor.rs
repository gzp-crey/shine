use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TextureTargetSize {
    /// Size matching the frame output
    Matching,
    /// Size propotional to the render target
    Propotional(f32, f32),
    /// Fixed sized
    Fixed(u32, u32),
}

/// Render target descriptor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TextureTargetDescriptor {
    pub format: wgpu::TextureFormat,
    pub size: TextureTargetSize,
    pub is_sampled: bool,
}

impl TextureTargetDescriptor {
    pub fn get_target_size(&self, frame_size: (u32, u32)) -> (u32, u32) {
        match &self.size {
            TextureTargetSize::Matching => frame_size,
            TextureTargetSize::Fixed(w, h) => (*w, *h),
            TextureTargetSize::Propotional(sw, sh) => {
                let w = ((frame_size.0 as f32) * sw).clamp(4., 65536.) as u32;
                let h = ((frame_size.1 as f32) * sh).clamp(4., 65536.) as u32;
                (w, h)
            }
        }
    }
}
