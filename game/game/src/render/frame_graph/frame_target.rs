use crate::{
    assets::FrameGraphDescriptor,
    render::{Compile, CompiledRenderTarget, FrameGraphError, FrameOutput, RenderTargetCompileExtra},
};

const FRAME_TARGET_INDEX: usize = usize::max_value();

pub struct FrameTarget {
    pub name: String,
    pub render_target: CompiledRenderTarget,
}

pub struct FrameTargets {
    frame_output: Option<FrameOutput>,
    targets: Vec<FrameTarget>,
}

impl FrameTargets {
    pub fn new() -> FrameTargets {
        FrameTargets {
            frame_output: None,
            targets: Vec::new(),
        }
    }

    pub fn get_texture_format(&self, target_index: usize) -> wgpu::TextureFormat {
        if target_index == FRAME_TARGET_INDEX {
            self.frame_output
                .as_ref()
                .map(|frame_output| frame_output.descriptor.format)
                .unwrap()
        } else {
            self.targets[target_index].render_target.format
        }
    }

    pub fn get_view(&self, target_index: usize) -> &wgpu::TextureView {
        if target_index == FRAME_TARGET_INDEX {
            self.frame_output
                .as_ref()
                .map(|frame_output| &frame_output.frame.view)
                .unwrap()
        } else {
            &self.targets[target_index].render_target.view
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

    pub fn find_target_index(&self, target_name: &str) -> Option<usize> {
        if target_name == "FRAME" {
            Some(FRAME_TARGET_INDEX)
        } else {
            self.targets.iter().position(|x| x.name == target_name)
        }
    }

    pub fn clear_targets(&mut self) {
        self.targets.clear();
    }

    pub fn recompile_targets(
        &mut self,
        device: &wgpu::Device,
        descriptor: &FrameGraphDescriptor,
    ) -> Result<(), FrameGraphError> {
        let mut targets = Vec::new();
        let frame_size = self.frame_size();

        for (name, target_desc) in &descriptor.targets {
            log::trace!("Creating render target {}", name);
            let compile_args = RenderTargetCompileExtra {
                frame_size: frame_size,
                is_sampled: descriptor.is_target_sampled(name),
            };
            let render_target = target_desc.compile(device, compile_args);
            targets.push(FrameTarget {
                name: name.clone(),
                render_target,
            });
        }

        self.targets = targets;
        Ok(())
    }
}
