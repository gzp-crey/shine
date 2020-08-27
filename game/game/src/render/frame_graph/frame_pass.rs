use crate::{
    assets::{FramePassDescriptor, PipelineStateDescriptor, RenderAttachementDescriptor, RenderSourceDescriptor},
    render::{
        frame_graph::{
            frame_target::{FrameTargetIndex, FrameTargetIndexRelocation, FrameTargets},
            render_pass::RenderPass,
        },
        Compile, RenderError,
    },
};

struct ResolvedFramePassOutput {
    color_target_indices: Vec<FrameTargetIndex>,
    depth_target_index: Option<FrameTargetIndex>,
    pipeline_state: PipelineStateDescriptor,
}

struct FramePassOutput {
    descriptor: RenderAttachementDescriptor,
    resolved: Option<ResolvedFramePassOutput>,
}

impl FramePassOutput {
    fn create(descriptor: RenderAttachementDescriptor) -> FramePassOutput {
        FramePassOutput {
            descriptor,
            resolved: None,
        }
    }

    fn release(&mut self) {
        self.resolved = None;
    }

    fn check_dirty_with_index_relocation(&mut self, targets: &FrameTargets) -> Result<bool, RenderError> {
        if let Some(resolved) = &mut self.resolved {
            if let Some(depth_target) = &mut resolved.depth_target_index {
                let depth_target_name = self.descriptor.depth.as_ref().map(|depth| &depth.target).unwrap();
                match targets.relocate_target_index(depth_target, &*depth_target_name) {
                    FrameTargetIndexRelocation::Current(index) => {
                        assert!(index == *depth_target);
                    }
                    FrameTargetIndexRelocation::Relocated(index) => {
                        log::debug!(
                            "FrameTarget {} index changed, but FramePassOutput is still valid (depth)",
                            depth_target_name,
                        );
                        *depth_target = index;
                    }
                    FrameTargetIndexRelocation::Changed(_) => {
                        log::debug!(
                            "FrameTarget {} changed, FramePassOutput re-compile required (depth)",
                            depth_target_name,
                        );
                        return Ok(true);
                    }
                    FrameTargetIndexRelocation::Removed => {
                        log::error!(
                            "FrameTarget {} removed, while a FramePassOutput still references it (depth)",
                            depth_target_name
                        );
                        return Err(RenderError::GraphInconsistency);
                    }
                };
            }

            for (color_idx, color_target) in resolved.color_target_indices.iter_mut().enumerate() {
                let color_target_name = &self.descriptor.colors[color_idx].target;
                match targets.relocate_target_index(color_target, &*color_target_name) {
                    FrameTargetIndexRelocation::Current(index) => {
                        assert!(index == *color_target);
                    }
                    FrameTargetIndexRelocation::Relocated(index) => {
                        log::debug!(
                            "FrameTarget {} index changed, but FramePassOutput is still valid (color({}))",
                            color_target_name,
                            color_idx
                        );
                        *color_target = index;
                    }
                    FrameTargetIndexRelocation::Changed(_) => {
                        log::debug!(
                            "FrameTarget {} changed, FramePassOutput re-compile required (color({}))",
                            color_target_name,
                            color_idx
                        );
                        return Ok(true);
                    }
                    FrameTargetIndexRelocation::Removed => {
                        log::error!(
                            "FrameTarget {} removed, while a FramePassOutput still references it (color({}))",
                            color_target_name,
                            color_idx
                        );
                        return Err(RenderError::GraphInconsistency);
                    }
                };
            }
            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn resolve(&mut self, targets: &FrameTargets) -> Result<(), RenderError> {
        if !self.check_dirty_with_index_relocation(targets)? {
            return Ok(());
        }

        // resolve depth
        let (depth_state, depth_target_index) = if let Some(depth) = &self.descriptor.depth {
            let target_index = targets.find_target_index(&depth.target).ok_or_else(|| {
                log::error!("FrameTarget {} not found forFramePassOutput (depth)", depth.target);
                RenderError::GraphInconsistency
            })?;

            let state = wgpu::DepthStencilStateDescriptor {
                format: targets.get_texture_format(&target_index),
                depth_write_enabled: depth.depth_operation.write_enabled,
                depth_compare: depth.depth_operation.compare,
                stencil: wgpu::StencilStateDescriptor {
                    front: depth.stencil_operation.front.clone(),
                    back: depth.stencil_operation.back.clone(),
                    read_mask: depth.stencil_operation.read_mask,
                    write_mask: depth.stencil_operation.write_mask,
                },
            };

            (Some(state), Some(target_index))
        } else {
            (None, None)
        };

        // resolve colors
        let mut color_target_indices = Vec::with_capacity(self.descriptor.colors.len());
        let mut color_states = Vec::with_capacity(self.descriptor.colors.len());
        for color in self.descriptor.colors.iter() {
            let target_index = targets.find_target_index(&color.target).ok_or_else(|| {
                log::error!("FrameTarget {} not found for FramePassOutput (color)", color.target);
                RenderError::GraphInconsistency
            })?;

            let state = wgpu::ColorStateDescriptor {
                format: targets.get_texture_format(&target_index),
                alpha_blend: color.alpha_blend.clone(),
                color_blend: color.color_blend.clone(),
                write_mask: color.write_mask,
            };

            color_target_indices.push(target_index);
            color_states.push(state);
        }

        self.resolved = Some(ResolvedFramePassOutput {
            color_target_indices,
            depth_target_index,
            pipeline_state: PipelineStateDescriptor {
                depth_state,
                color_states,
            },
        });
        Ok(())
    }
}

struct ResolvedFramePassInput {
    sampler: wgpu::Sampler,
    target_index: FrameTargetIndex,
}

/// Pass render inputs indexing the render targets
struct FramePassInput {
    descriptor: RenderSourceDescriptor,
    resolved: Option<ResolvedFramePassInput>,
}

impl FramePassInput {
    fn create(descriptor: RenderSourceDescriptor) -> FramePassInput {
        FramePassInput {
            descriptor,
            resolved: None,
        }
    }

    fn release(&mut self) {
        self.resolved = None;
    }

    fn check_dirty_with_index_relocation(&mut self, targets: &FrameTargets) -> Result<bool, RenderError> {
        if let Some(resolved) = &mut self.resolved {
            match targets.relocate_target_index(&resolved.target_index, &self.descriptor.target) {
                FrameTargetIndexRelocation::Current(index) => {
                    assert!(index == resolved.target_index);
                }
                FrameTargetIndexRelocation::Relocated(index) => {
                    log::debug!(
                        "FrameTarget {} index changed, but FramePassInput is still valid",
                        self.descriptor.target,
                    );
                    resolved.target_index = index;
                }
                FrameTargetIndexRelocation::Changed(_) => {
                    log::debug!(
                        "FrameTarget {} changed, FramePassInput re-compile required",
                        self.descriptor.target,
                    );
                    return Ok(true);
                }
                FrameTargetIndexRelocation::Removed => {
                    log::error!(
                        "FrameTarget {} removed, while a FramePassInput still references it",
                        self.descriptor.target,
                    );
                    return Err(RenderError::GraphInconsistency);
                }
            };

            Ok(false)
        } else {
            Ok(true)
        }
    }

    fn resolve(&mut self, device: &wgpu::Device, targets: &FrameTargets) -> Result<(), RenderError> {
        if !self.check_dirty_with_index_relocation(targets)? {
            return Ok(());
        }

        let target_index = targets.find_target_index(&self.descriptor.target).ok_or_else(|| {
            log::error!("FrameTarget {} not found for FramePassInput", self.descriptor.target);
            RenderError::GraphInconsistency
        })?;
        let sampler = self.descriptor.sampler.compile(device, ());
        self.resolved = Some(ResolvedFramePassInput { target_index, sampler });
        Ok(())
    }
}

pub struct FramePass {
    name: String,
    inputs: Vec<FramePassInput>,
    output: FramePassOutput,
    generation: usize,
}

impl FramePass {
    fn create(name: String, descriptor: FramePassDescriptor) -> FramePass {
        let FramePassDescriptor { inputs, output } = descriptor;
        let inputs = inputs.into_iter().map(|input| FramePassInput::create(input)).collect();
        let output = FramePassOutput::create(output);
        FramePass {
            name,
            inputs,
            output,
            generation: 0,
        }
    }

    fn release(&mut self) {
        for input in self.inputs.iter_mut() {
            input.release();
        }
        self.output.release();
    }

    fn resolve(&mut self, device: &wgpu::Device, targets: &FrameTargets) -> Result<(), RenderError> {
        for input in self.inputs.iter_mut() {
            input.resolve(device, targets)?;
        }
        self.output.resolve(targets)?;
        Ok(())
    }

    pub fn get_render_pass_descriptors(
        &self,
        targets: &FrameTargets,
    ) -> (
        Vec<wgpu::RenderPassColorAttachmentDescriptor<'_>>,
        Option<wgpu::RenderPassDepthStencilAttachmentDescriptor<'_>>,
        &'_ PipelineStateDescriptor,
    ) {
        unimplemented!()
    }
}

pub struct FramePasses {
    passes: Vec<FramePass>,
    generation: usize,
}

impl FramePasses {
    pub fn new() -> FramePasses {
        FramePasses {
            passes: Vec::new(),
            generation: 0,
        }
    }

    pub fn add_pass(&mut self, name: String, descriptor: FramePassDescriptor) -> Result<(), RenderError> {
        if self.find_pass_index(&name).is_some() {
            log::error!("FramePass {} alread exists", name);
            Err(RenderError::GraphInconsistency)
        } else {
            self.passes.push(FramePass::create(name, descriptor));
            Ok(())
        }
    }

    pub fn remove_pass(&mut self, name: &str) -> Result<(), RenderError> {
        let len = self.passes.len();
        self.passes.retain(|pass| pass.name != name);
        if len == self.passes.len() {
            log::error!("FramePass {} not found", name);
            Err(RenderError::GraphInconsistency)
        } else {
            Ok(())
        }
    }

    pub fn clear(&mut self) {
        self.generation += 1;
        self.passes.clear();
    }

    pub fn find_pass_index(&self, pass_name: &str) -> Option<usize> {
        self.passes.iter().position(|x| x.name == pass_name)
    }

    pub fn release(&mut self) {
        for pass in self.passes.iter_mut() {
            pass.release();
        }
    }

    pub fn resolve(&mut self, device: &wgpu::Device, targets: &FrameTargets) -> Result<(), RenderError> {
        for pass in self.passes.iter_mut() {
            pass.resolve(device, targets)?;
        }

        Ok(())
    }

    pub fn is_target_sampled(&self, target_name: &str) -> bool {
        for pass in self.passes.iter() {
            if pass.inputs.iter().any(|input| input.descriptor.target == target_name) {
                return true;
            }
        }
        false
    }

    pub fn begin_pass<'r, 'f: 'r, 'e: 'f>(
        &'f self,
        encoder: &'e mut wgpu::CommandEncoder,
        targets: &'f FrameTargets,
        pass_name: &'f str,
    ) -> Result<RenderPass<'r>, RenderError> {
        if let Some(pass) = self.passes.iter().find(|pass| pass.name == pass_name) {
            let depth_desc = if let Some(depth) = &pass.output.descriptor.depth {
                let target_index = pass
                    .output
                    .resolved
                    .as_ref()
                    .map(|resolved| resolved.depth_target_index.as_ref().unwrap())
                    .unwrap();
                Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: targets.get_view(target_index),
                    depth_ops: depth.depth_operation.operation,
                    stencil_ops: depth.stencil_operation.operation,
                })
            } else {
                None
            };

            let mut color_desc = Vec::new();
            for (id, color) in pass.output.descriptor.colors.iter().enumerate() {
                let target_index = pass
                    .output
                    .resolved
                    .as_ref()
                    .map(|resolved| &resolved.color_target_indices[id])
                    .unwrap();
                color_desc.push(wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: targets.get_view(target_index),
                    resolve_target: None,
                    ops: color.operation,
                });
            }

            let pipeline_state = pass
                .output
                .resolved
                .as_ref()
                .map(|resolved| &resolved.pipeline_state)
                .unwrap();

            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &color_desc[..],
                depth_stencil_attachment: depth_desc,
            });

            Ok(RenderPass::new(render_pass, pipeline_state, &targets))
        } else {
            //log::warn!("No [{}] pass was found", pass);
            Err(RenderError::MissingFramePass(pass_name.to_owned()))
        }
    }
}
