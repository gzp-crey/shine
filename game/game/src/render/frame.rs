use crate::{
    assets::{
        AssetError, AssetIO, FrameGraphDescriptor, FramePassDescriptor, PipelineStateDescriptor,
        RenderAttachementDescriptor, RenderSourceDescriptor, UniformScope, Url, UrlError,
    },
    render::{
        Compile, CompiledPipeline, CompiledRenderTarget, Context, PipelineBindGroup, RenderError,
        RenderTargetCompileExtra, Surface,
    },
};
use shine_ecs::core::async_task::AsyncTask;
use std::{
    ops::{Deref, DerefMut},
    sync::Mutex,
};

pub const DEFAULT_PASS: &str = "$";
const FRAME_TARGET_INDEX: usize = usize::max_value();

pub struct FrameOutput {
    pub frame: wgpu::SwapChainTexture,
    pub descriptor: wgpu::SwapChainDescriptor,
}

struct FrameTarget {
    pub name: String,
    pub render_target: CompiledRenderTarget,
}

struct FrameTargets {
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

struct PassDepthOutput {
    target_index: usize,
    depth_operation: Option<wgpu::Operations<f32>>,
    stencil_operation: Option<wgpu::Operations<u32>>,
}

struct PassColorOutput {
    target_index: usize,
    operation: wgpu::Operations<wgpu::Color>,
}

struct PassOutput {
    depth: Option<PassDepthOutput>,
    colors: Vec<PassColorOutput>,
    pipeline_state: PipelineStateDescriptor,
}

impl PassOutput {
    fn create(targets: &FrameTargets, descriptor: &RenderAttachementDescriptor) -> Result<PassOutput, FrameGraphError> {
        let (output_depth, stage_depth) = match &descriptor.depth {
            Some(depth) => {
                let depth_output = PassDepthOutput {
                    target_index: targets.find_target_index(&depth.target).ok_or(FrameGraphError)?,
                    depth_operation: depth.depth_operation.operation.clone(),
                    stencil_operation: depth.stencil_operation.operation.clone(),
                };
                let stage_depth = wgpu::DepthStencilStateDescriptor {
                    format: targets.get_texture_format(depth_output.target_index),
                    depth_write_enabled: depth.depth_operation.write_enabled,
                    depth_compare: depth.depth_operation.compare,
                    stencil: wgpu::StencilStateDescriptor {
                        front: depth.stencil_operation.front.clone(),
                        back: depth.stencil_operation.back.clone(),
                        read_mask: depth.stencil_operation.read_mask,
                        write_mask: depth.stencil_operation.write_mask,
                    },
                };
                (Some(depth_output), Some(stage_depth))
            }
            None => (None, None),
        };

        let mut stage_colors = Vec::with_capacity(descriptor.colors.len());
        let mut output_colors = Vec::with_capacity(descriptor.colors.len());
        for color in descriptor.colors.iter() {
            let output_color = PassColorOutput {
                target_index: targets.find_target_index(&color.target).ok_or(FrameGraphError)?,
                operation: color.operation,
            };
            let stage_color = wgpu::ColorStateDescriptor {
                format: targets.get_texture_format(output_color.target_index),
                alpha_blend: color.alpha_blend.clone(),
                color_blend: color.color_blend.clone(),
                write_mask: color.write_mask,
            };
            output_colors.push(output_color);
            stage_colors.push(stage_color);
        }

        Ok(PassOutput {
            depth: output_depth,
            colors: output_colors,
            pipeline_state: PipelineStateDescriptor {
                depth_state: stage_depth,
                color_states: stage_colors,
            },
        })
    }
}

/// Pass render inputs indexing the render targets
struct PassInput {
    texture_index: usize,
    sampler: wgpu::Sampler,
}

impl PassInput {
    fn create(
        device: &wgpu::Device,
        targets: &FrameTargets,
        descriptor: &RenderSourceDescriptor,
    ) -> Result<PassInput, FrameGraphError> {
        Ok(PassInput {
            texture_index: targets.find_target_index(&descriptor.target).ok_or(FrameGraphError)?,
            sampler: descriptor.sampler.compile(device, ()),
        })
    }
}

struct Pass {
    name: String,
    inputs: Vec<PassInput>,
    output: PassOutput,
}

impl Pass {
    fn create(
        device: &wgpu::Device,
        targets: &FrameTargets,
        name: &str,
        descriptor: &FramePassDescriptor,
    ) -> Result<Pass, FrameGraphError> {
        let inputs = descriptor
            .inputs
            .iter()
            .map(|input| PassInput::create(device, targets, input))
            .collect::<Result<Vec<_>, _>>()?;
        let output = PassOutput::create(targets, &descriptor.output)?;
        Ok(Pass {
            name: name.to_owned(),
            inputs,
            output,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FrameGraphError;

pub struct Frame {
    descriptor: Option<Result<FrameGraphDescriptor, FrameGraphLoadError>>,
    descriptor_loader: Option<AsyncTask<Result<FrameGraphDescriptor, FrameGraphLoadError>>>,

    targets: FrameTargets,
    passes: Result<Vec<Pass>, FrameGraphError>,

    commands: Mutex<Vec<wgpu::CommandBuffer>>,
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            descriptor: Some(Ok(FrameGraphDescriptor::single_pass())),
            descriptor_loader: None,
            targets: FrameTargets::new(),
            passes: Ok(Vec::new()),
            commands: Mutex::new(Vec::new()),
        }
    }

    pub fn load_graph(&mut self, assetio: AssetIO, descriptor: String) {
        self.passes = Ok(Vec::new());
        self.targets.clear_targets();
        let task = async move { assetio.load_frame_graph(descriptor).await };
        self.descriptor_loader = Some(AsyncTask::start(task));
        self.descriptor = None;
    }

    pub fn set_graph(&mut self, descriptor: Result<FrameGraphDescriptor, FrameGraphLoadError>) {
        self.passes = Ok(Vec::new());
        self.targets.clear_targets();
        self.descriptor_loader = None;
        self.descriptor = Some(descriptor);
    }

    fn recompile_frame_graph(&mut self, context: &Context) -> Result<(), FrameGraphError> {
        log::info!("Compiling frame graph");
        match &self.descriptor {
            Some(Ok(desc)) => {
                log::debug!("Creating render targets");
                self.targets.recompile_targets(context.device(), desc)?;

                log::debug!("Creating passes");
                let mut passes = Vec::new();
                for (name, pass_desc) in &desc.passes {
                    log::trace!("Creating pass {}", name);
                    let pass = Pass::create(context.device(), &self.targets, name, pass_desc)?;
                    passes.push(pass);
                }
                self.passes = Ok(passes);
                Ok(())
            }
            Some(Err(_)) => Err(FrameGraphError),
            None => {
                // for explicit setting, descriptor is always some
                // for async loaded, recompile should not be called until desc's been loaded
                unreachable!("Missing frame graph descriptor");
            }
        }
    }

    fn try_get_new_descriptor(&mut self) -> Option<Result<FrameGraphDescriptor, FrameGraphLoadError>> {
        if let Some(loader) = self.descriptor_loader.as_mut() {
            log::error!("Checking loader for frame graph");
            match loader.try_get() {
                Err(_) => {
                    log::warn!("Frame graph load canceled");
                    Some(Err(FrameGraphLoadError::Canceled))
                }
                Ok(Some(Err(err))) => {
                    log::warn!("Frame graph load failed: {:?}", err);
                    Some(Err(err))
                }
                Ok(Some(Ok(descriptor))) => Some(Ok(descriptor)),
                Ok(None) => None,
            }
        } else {
            None
        }
    }

    pub fn start_frame(&mut self, surface: &Surface, context: &mut Context) -> Result<(), RenderError> {
        if let Some(descriptor) = self.try_get_new_descriptor() {
            self.set_graph(descriptor);
        }
        if self.descriptor.is_none() {
            return Err(RenderError::GraphNotReady);
        }
        if self.passes.is_err() {
            return Err(RenderError::GraphError);
        }

        let frame = context.create_frame(surface)?;
        self.targets.set_frame_output(Some(frame));

        if self.passes.as_ref().map(|p| p.is_empty()).unwrap() {
            if let Err(err) = self.recompile_frame_graph(context) {
                log::warn!("Failed to compile frame graph: {:?}", err);
                {
                    // todo: is it ok to clear the render queue ???
                    let mut commands = self.commands.lock().unwrap();
                    commands.clear();
                }
                self.targets.clear_targets();
                self.targets.set_frame_output(None);
                self.passes = Err(err);
                return Err(RenderError::GraphError);
            }
        } else {
            // check frame size, resize targets
        }

        Ok(())
    }

    pub fn end_frame(&mut self, queue: &wgpu::Queue) {
        {
            let mut commands = self.commands.lock().unwrap();
            queue.submit(commands.drain(..));
        }
        self.targets.set_frame_output(None);
    }

    pub fn frame_output(&self) -> Option<&FrameOutput> {
        self.targets.frame_output()
    }

    pub fn frame_size(&self) -> (u32, u32) {
        self.targets.frame_size()
    }

    pub fn begin_pass<'r, 'f: 'r, 'e: 'f>(
        &'f self,
        encoder: &'e mut wgpu::CommandEncoder,
        pass_name: &'f str,
    ) -> Result<RenderPass<'r>, RenderError> {
        if let Ok(passes) = &self.passes {
            if let Some(pass) = passes.iter().find(|x| x.name == pass_name) {
                Ok(RenderPass::new(pass, &self.targets, &self.commands, encoder))
            } else {
                //log::warn!("No [{}] pass was found", pass);
                Err(RenderError::MissingPass(pass_name.to_owned()))
            }
        } else {
            Err(RenderError::GraphError)
        }
    }

    pub fn add_command(&self, command: wgpu::CommandBuffer) {
        let mut commands = self.commands.lock().unwrap();
        commands.push(command);
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

/// Error during frame graph load
#[derive(Debug)]
pub enum FrameGraphLoadError {
    Asset(AssetError),
    Canceled,
}

impl From<UrlError> for FrameGraphLoadError {
    fn from(err: UrlError) -> FrameGraphLoadError {
        FrameGraphLoadError::Asset(AssetError::InvalidUrl(err))
    }
}

impl From<AssetError> for FrameGraphLoadError {
    fn from(err: AssetError) -> FrameGraphLoadError {
        FrameGraphLoadError::Asset(err)
    }
}

impl From<bincode::Error> for FrameGraphLoadError {
    fn from(err: bincode::Error) -> FrameGraphLoadError {
        FrameGraphLoadError::Asset(AssetError::ContentLoad(format!("Binary stream error: {}", err)))
    }
}

impl AssetIO {
    async fn load_frame_graph(&self, source_id: String) -> Result<FrameGraphDescriptor, FrameGraphLoadError> {
        let url = Url::parse(&source_id)?;
        log::debug!("[{:?}] Loading frame graph...", source_id);
        let data = self.download_binary(&url).await?;

        let descriptor = bincode::deserialize::<FrameGraphDescriptor>(&data)?;
        log::trace!("Graph: {:#?}", descriptor);
        descriptor.check_target_references()?;

        Ok(descriptor)
    }
}

pub struct RenderPass<'r> {
    pass: &'r Pass,
    targets: &'r FrameTargets,
    commands: &'r Mutex<Vec<wgpu::CommandBuffer>>,
    pub render_pass: wgpu::RenderPass<'r>,
}

impl<'r> RenderPass<'r> {
    fn new<'f: 'r>(
        pass: &'f Pass,
        targets: &'f FrameTargets,
        commands: &'f Mutex<Vec<wgpu::CommandBuffer>>,
        encoder: &'f mut wgpu::CommandEncoder,
    ) -> RenderPass<'r> {
        // Pass is given, use the attached output(s)
        let color_desc = pass
            .output
            .colors
            .iter()
            .map(|attachement| wgpu::RenderPassColorAttachmentDescriptor {
                attachment: targets.get_view(attachement.target_index),
                resolve_target: None,
                ops: attachement.operation,
            })
            .collect::<Vec<_>>();

        let depth_desc =
            pass.output
                .depth
                .as_ref()
                .map(|attachement| wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    attachment: targets.get_view(attachement.target_index),
                    depth_ops: attachement.depth_operation,
                    stencil_ops: attachement.stencil_operation,
                });

        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &color_desc[..],
            depth_stencil_attachment: depth_desc,
        });

        RenderPass {
            pass,
            targets,
            commands,
            render_pass,
        }
    }

    pub fn get_pipeline_state(&self) -> &PipelineStateDescriptor {
        &self.pass.output.pipeline_state
    }

    pub fn set_pipeline(&mut self, pipeline: &'r CompiledPipeline, bindings: &'r PipelineBindGroup) {
        self.render_pass.set_pipeline(&pipeline.pipeline);
        self.render_pass
            .set_bind_group(UniformScope::Auto.bind_location(), &bindings.auto, &[]);
        self.render_pass
            .set_bind_group(UniformScope::Global.bind_location(), &bindings.global, &[]);
        self.render_pass
            .set_bind_group(UniformScope::Local.bind_location(), &bindings.local, &[]);
    }
}

impl<'r> Deref for RenderPass<'r> {
    type Target = wgpu::RenderPass<'r>;

    fn deref(&self) -> &Self::Target {
        &self.render_pass
    }
}

impl<'r> DerefMut for RenderPass<'r> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.render_pass
    }
}
