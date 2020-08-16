use crate::{
    assets::{
        AssetError, AssetIO, FrameGraphDescriptor, FramePassDescriptor, PipelineStateDescriptor,
        RenderAttachementDescriptor, RenderSourceDescriptor, Url, UrlError,
    },
    render::{Compile, CompiledRenderTarget, Context, RenderTargetCompileExtra, Surface},
    GameError,
};
use shine_ecs::core::async_task::AsyncTask;
use std::{borrow::Cow, sync::Mutex};

pub const DEFAULT_PASS: &str = "$";
const FRAME_TARGET_INDEX: usize = usize::max_value();

pub struct FrameOutput {
    pub frame: wgpu::SwapChainTexture,
    pub descriptor: wgpu::SwapChainDescriptor,
}

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

pub struct FrameTextures<'t> {
    pub frame: &'t wgpu::SwapChainTexture,
    pub textures: Vec<(&'t FrameTarget, &'t wgpu::Sampler)>,
}

#[derive(Debug, Clone)]
pub struct FrameGraphError;

#[derive(Debug, Clone)]
pub enum FrameStartError {
    /// Failed to crate frame output
    Output,

    /// Frame Graph has some issue
    Graph(FrameGraphError),

    /// Frame is not ready
    Pending,
}

pub struct Frame {
    descriptor: Option<FrameGraphDescriptor>,
    descriptor_loader: Option<AsyncTask<Result<FrameGraphDescriptor, FrameGraphLoadError>>>,

    targets: FrameTargets,
    passes: Result<Vec<Pass>, FrameGraphError>,

    buffers: Mutex<Vec<wgpu::CommandBuffer>>,
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            descriptor: Some(FrameGraphDescriptor::single_pass()),
            descriptor_loader: None,
            targets: FrameTargets::new(),
            passes: Ok(Vec::new()),
            buffers: Mutex::new(Vec::new()),
        }
    }

    pub fn load_graph(&mut self, _context: &Context, assetio: AssetIO, descriptor: String) {
        let task = async move { assetio.load_frame_graph(descriptor).await };

        // release graph and start async loading.
        self.release_frame_graph();
        self.descriptor = None;
        self.descriptor_loader = Some(AsyncTask::start(task));
    }

    pub fn set_graph(&mut self, context: &Context, descriptor: Result<FrameGraphDescriptor, FrameGraphError>) {
        self.descriptor_loader = None;
        self.release_frame_graph();
        match descriptor {
            Ok(descriptor) => {
                self.descriptor = Some(descriptor);
                self.recompile_frame_graph(context);
            }
            Err(err) => {
                self.descriptor = None;
                self.passes = Err(err);
            }
        };
    }

    fn release_frame_graph(&mut self) {
        log::info!("Releasing frame graph");
        self.passes = Ok(Vec::new());
        self.targets.clear_targets();
    }

    fn recompile_frame_graph(&mut self, context: &Context) {
        log::info!("Compiling frame graph");
        match &self.descriptor {
            Some(desc) => {
                log::debug!("Creating render targets");
                if let Err(err) = self.targets.recompile_targets(context.device(), desc) {
                    log::warn!("Frame graph compilation failed: {:?}", err);
                    self.passes = Err(FrameGraphError);
                    return;
                }

                log::debug!("Creating passes");
                let mut passes = Vec::new();
                for (name, pass_desc) in &desc.passes {
                    log::trace!("Creating pass {}", name);
                    let pass = match Pass::create(context.device(), &self.targets, name, pass_desc) {
                        Err(err) => {
                            log::warn!("Frame graph compilation failed: {:?}", err);
                            self.passes = Err(FrameGraphError);
                            return;
                        }
                        Ok(pass) => {
                            passes.push(pass);
                        }
                    };
                }
                self.passes = Ok(passes);
            }
            None => {
                // for explicit setting, descriptor is always some
                // for async loaded, recompile should not be called until desc's been loaded
                unreachable!("Missing frame graph descriptor");
            }
        }
    }

    pub fn start_frame(&mut self, surface: &Surface, context: &mut Context) -> Result<(), FrameStartError> {
        let frame = context.create_frame(surface).map_err(|err| FrameStartError::Output)?;
        self.targets.set_frame_output(Some(frame));

        if let Some(loader) = self.descriptor_loader.as_mut() {
            log::error!("Checking loader");
            let descriptor = match loader.try_get() {
                Err(err) => {
                    log::warn!("Frame graph load failed error");
                    Some(Err(FrameGraphError))
                }
                Ok(Some(Err(err))) => {
                    log::warn!("Frame graph load failed: {:?}", err);
                    Some(Err(FrameGraphError))
                }
                Ok(Some(Ok(descriptor))) => Some(Ok(descriptor)),
                Ok(None) => None,
            };

            if let Some(descriptor) = descriptor {
                self.set_graph(context, descriptor);
            }
        };

        // check frame size, resize targets

        match &self.passes {
            Err(err) => Err(FrameStartError::Graph(err.clone())),
            Ok(pass) if pass.is_empty() => Err(FrameStartError::Pending),
            Ok(_) => Ok(()),
        }
    }

    pub fn postprocess_frame(&mut self) {}

    pub fn end_frame(&mut self, queue: &wgpu::Queue) {
        {
            let mut buffers = self.buffers.lock().unwrap();
            queue.submit(buffers.drain(..));
        }
        self.targets.set_frame_output(None);
    }

    pub fn frame_output(&self) -> Option<&FrameOutput> {
        self.targets.frame_output()
    }

    pub fn frame_size(&self) -> (u32, u32) {
        self.targets.frame_size()
    }

    /*pub fn pass_textures(&self, pass: &str) -> Result<FrameTextures<'_>, GameError> {
        if pass == "DEBUG" {
            Ok(FrameTextures {
                frame: &self.frame_output().unwrap().frame,
                textures: Vec::new(),
            })
        } else if let Some(pass) = self.passes.iter().find(|x| x.name == pass) {
            Ok(FrameTextures {
                frame: &self.frame_output().unwrap().frame,
                textures: pass
                    .inputs
                    .iter()
                    .map(|texture| {
                        (
                            self.targets.get_target(texture.texture_index).unwrap(),
                            &texture.sampler,
                        )
                    })
                    .collect(),
            })
        } else {
            //log::warn!("No [{}] pass was found", pass);
            Err(GameError::Render(format!("No [{}] pass was found", pass)))
        }
    }*/

    pub fn get_pipeline_state(&self, pass_name: &str) -> Result<&PipelineStateDescriptor, GameError> {
        if let Ok(passes) = &self.passes {
            if let Some(pass) = passes.iter().find(|x| x.name == pass_name) {
                Ok(&pass.output.pipeline_state)
            } else {
                //log::warn!("No [{}] pass was found", pass);
                Err(GameError::Render(format!("No [{}] pass was found", pass_name)))
            }
        } else {
            Err(GameError::Render(format!("graph error")))
        }
    }

    pub fn create_pass<'e, 'f: 'e>(
        &'f self,
        encoder: &'f mut wgpu::CommandEncoder,
        pass_name: &str,
    ) -> Result<wgpu::RenderPass<'e>, GameError> {
        // Pass is given, use the attached output(s)
        if let Ok(passes) = &self.passes {
            if let Some(pass) = passes.iter().find(|x| x.name == pass_name) {
                let color_desc = pass
                    .output
                    .colors
                    .iter()
                    .map(|attachement| wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: self.targets.get_view(attachement.target_index),
                        resolve_target: None,
                        ops: attachement.operation,
                    })
                    .collect::<Vec<_>>();

                let depth_desc =
                    pass.output
                        .depth
                        .as_ref()
                        .map(|attachement| wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: self.targets.get_view(attachement.target_index),
                            depth_ops: attachement.depth_operation,
                            stencil_ops: attachement.stencil_operation,
                        });

                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &color_desc[..],
                    depth_stencil_attachment: depth_desc,
                });

                Ok(render_pass)
            } else {
                //log::warn!("No [{}] pass was found", pass);
                Err(GameError::Render(format!("No [{}] pass was found", pass_name)))
            }
        } else {
            Err(GameError::Render(format!("graph error")))
        }
    }

    pub fn add_command(&self, commands: wgpu::CommandBuffer) {
        let mut buffers = self.buffers.lock().unwrap();
        buffers.push(commands);
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
