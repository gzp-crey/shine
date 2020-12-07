use crate::assets::Uniform;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ViewProj {
    pub mx: [f32; 16],
}

unsafe impl bytemuck::Pod for ViewProj {}
unsafe impl bytemuck::Zeroable for ViewProj {}

impl Uniform for ViewProj {
    /*fn size() -> usize {
        mem::size_of::<Self>()
    }*/
}
