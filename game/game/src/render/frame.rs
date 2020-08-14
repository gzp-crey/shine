use crate::{
    assets::{
        AssetError, AssetIO, FrameGraphDescriptor, FramePassDescriptor, RenderAttachementDescriptor,
        RenderSourceDescriptor, Url, UrlError,
    },
    render::{Compile, CompiledRenderTarget, Context, RenderTargetCompileExtra, Surface},
    GameError,
};
use shine_ecs::core::async_task::AsyncTask;
use std::{borrow::Cow, sync::Mutex};

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

    pub fn get_view(&self, target_index: usize) -> &wgpu::TextureView {
        if target_index == FRAME_TARGET_INDEX {
            let frame = &self.frame_output.as_ref().unwrap().frame;
            &frame.view
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
        assert!(self.targets.is_empty());
        for (name, target_desc) in &descriptor.targets {
            log::trace!("Creating render target {}", name);
            let compile_args = RenderTargetCompileExtra {
                frame_size: self.frame_size(),
                is_sampled: descriptor.is_target_sampled(name),
            };
            let render_target = target_desc.compile(device, compile_args);
            self.targets.push(FrameTarget {
                name: name.clone(),
                render_target,
            });
        }

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
}

impl PassOutput {
    fn create(targets: &FrameTargets, descriptor: &RenderAttachementDescriptor) -> Result<PassOutput, FrameGraphError> {
        let depth = descriptor
            .depth
            .as_ref()
            .map(|depth| {
                Ok(PassDepthOutput {
                    target_index: targets.find_target_index(&depth.target).ok_or(FrameGraphError)?,
                    depth_operation: depth.depth_operation.as_ref().map(|op| op.operation),
                    stencil_operation: depth.stencil_operation.as_ref().map(|op| op.operation),
                })
            })
            .transpose()?;

        let colors = descriptor
            .colors
            .iter()
            .map(|color| {
                Ok(PassColorOutput {
                    target_index: targets.find_target_index(&color.target).ok_or(FrameGraphError)?,
                    operation: color.operation,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(PassOutput { depth, colors })
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
    outputs: PassOutput,
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
        let outputs = PassOutput::create(targets, &descriptor.output)?;
        Ok(Pass {
            name: name.to_owned(),
            inputs,
            outputs,
        })
    }
}

pub struct FrameTextures<'t> {
    pub frame: &'t wgpu::SwapChainTexture,
    pub textures: Vec<(&'t FrameTarget, &'t wgpu::Sampler)>,
}

#[derive(Debug, Clone)]
pub struct FrameGraphError;

pub struct Frame {
    descriptor: Result<Option<FrameGraphDescriptor>, FrameGraphError>,
    descriptor_loader: Option<AsyncTask<Result<FrameGraphDescriptor, FrameGraphLoadError>>>,

    targets: FrameTargets,
    passes: Vec<Pass>,

    buffers: Mutex<Vec<wgpu::CommandBuffer>>,
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            descriptor: Ok(None),
            descriptor_loader: None,
            targets: FrameTargets::new(),
            passes: Vec::new(),
            buffers: Mutex::new(Vec::new()),
        }
    }

    pub fn load_graph(&mut self, assetio: AssetIO, descriptor: String) {
        let task = async move { assetio.load_frame_graph(descriptor).await };
        self.descriptor_loader = Some(AsyncTask::start(task));
    }

    pub fn set_graph(&mut self, context: &Context, descriptor: Option<FrameGraphDescriptor>) {
        self.set_frame_graph(context, Ok(descriptor));
    }

    fn release_frame_graph(&mut self) {
        log::info!("Releasing frame graph");
        self.passes.clear();
        self.targets.clear_targets();
    }

    fn recompile_frame_graph(&mut self, context: &Context) -> Result<(), FrameGraphError> {
        log::info!("Compiling frame graph");
        match &self.descriptor {
            Ok(Some(desc)) => {
                log::debug!("Creating render targets");
                self.targets.recompile_targets(context.device(), desc)?;

                log::debug!("Creating passes");
                for (name, pass_desc) in &desc.passes {
                    log::trace!("Creating pass {}", name);
                    let pass = Pass::create(context.device(), &self.targets, name, pass_desc)?;
                    self.passes.push(pass);
                }
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(err) => Err(err.clone()),
        }
    }

    fn set_frame_graph(
        &mut self,
        context: &Context,
        descriptor: Result<Option<FrameGraphDescriptor>, FrameGraphError>,
    ) {
        self.descriptor_loader = None;
        self.descriptor = descriptor;

        self.release_frame_graph();
        if let Err(err) = self.recompile_frame_graph(context) {
            self.descriptor = Err(err)
        }
    }

    pub fn start_frame(&mut self, surface: &Surface, context: &mut Context) -> Result<(), GameError> {
        let frame = context.create_frame(surface)?;
        self.targets.set_frame_output(Some(frame));

        if let Some(loader) = self.descriptor_loader.as_mut() {
            log::error!("Checking loader");
            match loader.try_get() {
                Err(_) => self.set_frame_graph(context, Err(FrameGraphError)),
                Ok(Some(Ok(descriptor))) => self.set_frame_graph(context, Ok(Some(descriptor))),
                Ok(Some(Err(err))) => {
                    log::warn!("Frame graph load failed: {:?}", err);
                    self.set_frame_graph(context, Err(FrameGraphError));
                }
                Ok(None) => {}
            }
        };

        // check frame size, resize targets
        Ok(())
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

    pub fn create_pass<'e, 'f: 'e>(
        &'f self,
        encoder: &'f mut wgpu::CommandEncoder,
        pass: Option<&str>,
    ) -> Result<wgpu::RenderPass<'e>, GameError> {
        match pass {
            None => {
                // Pass is not given, use frame as output
                let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: Cow::Borrowed(&[wgpu::RenderPassColorAttachmentDescriptor {
                        attachment: self.targets.get_view(FRAME_TARGET_INDEX),
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                            store: true,
                        },
                    }]),
                    depth_stencil_attachment: None,
                });

                Ok(pass)
            }

            Some(pass_name) => {
                // Pass is given, use the attached output(s)
                if let Some(pass) = self.passes.iter().find(|x| x.name == pass_name) {
                    let color_desc = pass
                        .outputs
                        .colors
                        .iter()
                        .map(|attachement| wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: self.targets.get_view(attachement.target_index),
                            resolve_target: None,
                            ops: attachement.operation,
                        })
                        .collect::<Vec<_>>();

                    let depth_desc = pass.outputs.depth.as_ref().map(|attachement| {
                        wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: self.targets.get_view(attachement.target_index),
                            depth_ops: attachement.depth_operation,
                            stencil_ops: attachement.stencil_operation,
                        }
                    });

                    let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: Cow::Borrowed(&color_desc[..]),
                        depth_stencil_attachment: depth_desc,
                    });

                    Ok(render_pass)
                } else {
                    //log::warn!("No [{}] pass was found", pass);
                    Err(GameError::Render(format!("No [{}] pass was found", pass_name)))
                }
            }
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
