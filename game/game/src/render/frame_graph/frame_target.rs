use crate::{
    assets::RenderTargetDescriptor,
    render::{
        frame_graph::frame_pass::FramePasses, Compile, CompiledRenderTarget, FrameOutput, RenderError,
        RenderTargetCompileExtra,
    },
};
use std::collections::HashMap;

struct ResolvedFrameTarget {
    render_target: CompiledRenderTarget,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameTargetIndex {
    store_generation: usize,
    target_generation: usize,
    index: usize,
}

impl FrameTargetIndex {
    pub fn is_frame_output(&self) -> bool {
        self.index == 0
    }
}

pub enum FrameTargetIndexRelocation {
    Current(FrameTargetIndex),
    Relocated(FrameTargetIndex),
    Changed(FrameTargetIndex),
    Removed,
}

struct FrameTarget {
    name: String,
    descriptor: RenderTargetDescriptor,

    generation: usize,
    resolved: Option<ResolvedFrameTarget>,
}

impl FrameTarget {
    fn create(name: String, descriptor: RenderTargetDescriptor, initial_generation: usize) -> FrameTarget {
        FrameTarget {
            name,
            descriptor,
            generation: initial_generation,
            resolved: None,
        }
    }

    fn release(&mut self) {
        self.resolved = None;
    }

    fn is_dirty(&self, frame_size: (u32, u32), is_sampled: bool) -> bool {
        if let Some(resolved) = &self.resolved {
            let size = self.descriptor.get_target_size(frame_size);

            resolved.render_target.size.0 != size.0
                || resolved.render_target.size.1 != size.1
                || resolved.render_target.is_sampled != is_sampled
        } else {
            false
        }
    }

    fn resolve(&mut self, device: &wgpu::Device, frame_size: (u32, u32), is_sampled: bool) -> Result<(), RenderError> {
        if self.is_dirty(frame_size, is_sampled) {
            let compile_args = RenderTargetCompileExtra {
                frame_size: frame_size,
                is_sampled: is_sampled,
            };
            let render_target = self.descriptor.compile(device, compile_args);
            self.generation += 1;
            self.resolved = Some(ResolvedFrameTarget { render_target });
        }
        Ok(())
    }
}

pub struct FrameTargets {
    frame_output: Option<FrameOutput>,
    /// for removed targets we keep track of the initial generation to avoid ABA issues
    /// If a target is added with the same name, start it's generation from where the removed stopped.
    target_generation_start: HashMap<String, usize>,
    targets: Vec<FrameTarget>,
    generation: usize,
}

impl FrameTargets {
    pub fn new() -> FrameTargets {
        FrameTargets {
            frame_output: None,
            target_generation_start: HashMap::new(),
            targets: Vec::new(),
            generation: 0,
        }
    }

    pub fn set_frame_output(&mut self, frame_output: Option<FrameOutput>) {
        self.frame_output = frame_output;
    }

    pub fn frame_output(&self) -> Option<&FrameOutput> {
        self.frame_output.as_ref()
    }

    pub fn frame_size(&self) -> (u32, u32) {
        self.frame_output()
            .map(|x| (x.descriptor.width, x.descriptor.height))
            .unwrap_or((0, 0))
    }

    pub fn add_target(&mut self, name: String, descriptor: RenderTargetDescriptor) -> Result<(), RenderError> {
        if self.find_target_index(&name).is_some() {
            log::error!("FrameTarget {} alread exists", name);
            Err(RenderError::GraphInconsistency)
        } else if name == "FRAME" {
            log::error!("FrameTarget {} alread exists, it is the surface output", name);
            Err(RenderError::GraphInconsistency)
        } else {
            self.generation += 1;
            let target_generation = self.target_generation_start.remove(&name).unwrap_or(0);
            self.targets
                .push(FrameTarget::create(name, descriptor, target_generation));
            Ok(())
        }
    }

    pub fn remove_target(&mut self, name: &str) -> Result<(), RenderError> {
        let len = self.targets.len();
        let target_generation_start = &mut self.target_generation_start;
        self.targets.retain(|target| {
            if target.name != name {
                // keep the genertion of the removed targets, thus if a target is added with the
                // same name, the clients will be informed that, it is not the same
                target_generation_start.insert(target.name.to_owned(), target.generation + 1);
                false
            } else {
                true
            }
        });

        if len == self.targets.len() {
            log::error!("FrameTarget {} not found", name);
            Err(RenderError::GraphInconsistency)
        } else {
            self.generation += 1;
            Ok(())
        }
    }

    pub fn clear_targets(&mut self) {
        self.targets.clear();
        self.generation += 1;
    }

    pub fn release(&mut self) {
        for target in self.targets.iter_mut() {
            target.release();
        }
    }

    pub fn resolve(&mut self, device: &wgpu::Device, passes: &FramePasses) -> Result<(), RenderError> {
        let frame_size = self.frame_size();

        for target in self.targets.iter_mut() {
            log::trace!("Resolving render target {}", target.name);
            let is_sampled = passes.is_target_sampled(&target.name);
            target.resolve(device, frame_size, is_sampled)?;
        }

        Ok(())
    }

    pub fn find_target_index(&self, target_name: &str) -> Option<FrameTargetIndex> {
        if target_name == "FRAME" {
            Some(FrameTargetIndex {
                store_generation: 0,
                target_generation: 0,
                index: 0,
            })
        } else {
            self.targets
                .iter()
                .position(|x| x.name == target_name)
                .map(|index| FrameTargetIndex {
                    store_generation: self.generation,
                    target_generation: self.targets[index].generation,
                    index: index + 1,
                })
        }
    }

    pub fn relocate_target_index(&self, target_index: &FrameTargetIndex, name: &str) -> FrameTargetIndexRelocation {
        if target_index.is_frame_output() {
            // ignore any change, assume a fixed format
            FrameTargetIndexRelocation::Current(target_index.clone())
        } else if target_index.store_generation != self.generation {
            // index into vec might be invalid
            if let Some(new_index) = self.find_target_index(name) {
                assert!(new_index.store_generation == self.generation);
                let target = &self.targets[new_index.index - 1];
                if target.generation != target_index.target_generation {
                    FrameTargetIndexRelocation::Changed(new_index)
                } else {
                    FrameTargetIndexRelocation::Relocated(new_index)
                }
            } else {
                FrameTargetIndexRelocation::Removed
            }
        } else {
            // index is valid, check for target change
            let target = &self.targets[target_index.index - 1];
            if target.generation != target_index.target_generation {
                FrameTargetIndexRelocation::Changed(FrameTargetIndex {
                    store_generation: target_index.store_generation,
                    target_generation: target.generation,
                    index: target_index.index,
                })
            } else {
                FrameTargetIndexRelocation::Current(target_index.clone())
            }
        }
    }

    pub fn get_texture_format(&self, target_index: &FrameTargetIndex) -> wgpu::TextureFormat {
        if target_index.is_frame_output() {
            self.frame_output
                .as_ref()
                .map(|frame_output| frame_output.descriptor.format)
                .unwrap()
        } else {
            assert!(target_index.store_generation == self.generation);
            let target = &self.targets[target_index.index - 1];
            assert!(target_index.target_generation == target.generation);
            target
                .resolved
                .as_ref()
                .map(|resolved| resolved.render_target.format)
                .unwrap()
        }
    }

    pub fn get_view(&self, target_index: &FrameTargetIndex) -> &wgpu::TextureView {
        if target_index.is_frame_output() {
            self.frame_output
                .as_ref()
                .map(|frame_output| &frame_output.frame.view)
                .unwrap()
        } else {
            assert!(target_index.store_generation == self.generation);
            let target = &self.targets[target_index.index - 1];
            assert!(target_index.target_generation == target.generation);
            target
                .resolved
                .as_ref()
                .map(|resolved| &resolved.render_target.view)
                .unwrap()
        }
    }
}
