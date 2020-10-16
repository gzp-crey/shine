use crate::{
    assets::TextureTargetDescriptor,
    render::{Compile, CompiledTextureTarget, RenderError, TextureTargetCompile},
};
use shine_ecs::resources::{Tag, TagMut};

struct ResolvedTextureTarget {
    render_target: CompiledTextureTarget,
}

pub struct TextureTarget {
    descriptor: TextureTargetDescriptor,
    generation: usize,
    resolved: Option<ResolvedTextureTarget>,
}

impl TextureTarget {
    pub fn from_descriptor(descriptor: TextureTargetDescriptor) -> Self {
        Self {
            descriptor,
            generation: 0,
            resolved: None,
        }
    }

    pub fn generation(&self) -> usize {
        self.generation
    }

    pub fn descriptor(&self) -> &TextureTargetDescriptor {
        &self.descriptor
    }

    fn is_dirty(&self, frame_size: (u32, u32)) -> bool {
        if let Some(resolved) = &self.resolved {
            let size = self.descriptor.get_target_size(frame_size);
            resolved.render_target.size.0 != size.0 || resolved.render_target.size.1 != size.1
        } else {
            false
        }
    }

    pub fn release(&mut self) {
        self.generation += 1;
        self.resolved = None;
    }

    pub fn resolve(&mut self, device: &wgpu::Device, frame_size: (u32, u32)) -> Result<(), RenderError> {
        if self.is_dirty(frame_size) {
            let render_target = TextureTargetCompile {
                descriptor: &self.descriptor,
                frame_size,
            }
            .compile(device);
            self.generation += 1;
            self.resolved = Some(ResolvedTextureTarget { render_target });
        }
        Ok(())
    }
}

pub type TextureTargetsRes<'a> = Tag<'a, TextureTarget>;
pub type TextureTargetsResMut<'a> = TagMut<'a, TextureTarget>;
