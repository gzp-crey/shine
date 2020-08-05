use crate::{
    assets::{AssetError, AssetIO, FrameGraphDescriptor, Url, UrlError},
    render::{Compile, CompiledRenderTarget, Context, RenderTargetCompileExtra, Surface},
    GameError,
};
use shine_ecs::core::async_task::AsyncTask;
use std::{borrow::Cow, sync::Mutex};

pub struct FrameOutput {
    pub frame: wgpu::SwapChainTexture,
    pub descriptor: wgpu::SwapChainDescriptor,
}

pub struct FrameTarget {
    pub name: String,
    pub render_target: CompiledRenderTarget,
}

/// Pass render target indexing the render targets
struct PassOutput {
    depth: Option<(usize, Option<wgpu::Operations<f32>>, Option<wgpu::Operations<u32>>)>,
    colors: Vec<(usize, wgpu::Operations<wgpu::Color>)>,
}

/// Pass render inputs indexing the render targets
struct PassInput {
    texture_index: usize,
    sampler: wgpu::Sampler,
}

struct Pass {
    name: String,
    inputs: Vec<PassInput>,
    outputs: PassOutput,
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

    frame_output: Option<FrameOutput>,
    targets: Vec<FrameTarget>,
    passes: Vec<Pass>,

    buffers: Mutex<Vec<wgpu::CommandBuffer>>,
}

impl Frame {
    pub fn new() -> Frame {
        Frame {
            descriptor: Ok(None),
            descriptor_loader: None,
            frame_output: None,
            targets: Vec::new(),
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

    fn find_target_index(&self, target_name: &str) -> Option<usize> {
        if target_name == "FRAME" {
            Some(usize::max_value())
        } else {
            self.targets.iter().position(|x| x.name == target_name)
        }
    }

    fn release_frame_graph(&mut self) {
        log::info!("Releasing frame graph");
        self.targets.clear();
        self.passes.clear();
    }

    fn compile_frame_graph(&mut self, context: &Context) -> Result<(), FrameGraphError> {
        log::info!("Compiling frame graph");
        match &self.descriptor {
            Ok(Some(desc)) => {
                for (name, target) in &desc.targets {
                    log::trace!("Creating render target {}", name);
                    let compile_args = RenderTargetCompileExtra {
                        frame_size: self.frame_size(),
                        is_sampled: desc.is_target_sampled(&name),
                    };
                    let render_target = target.compile(context.device(), compile_args);
                    self.targets.push(FrameTarget {
                        name: name.clone(),
                        render_target,
                    })
                }

                for (name, pass) in &desc.passes {
                    log::trace!("Creating pass {}", name);
                    let inputs = pass
                        .inputs
                        .iter()
                        .map(|input| {
                            Ok(PassInput {
                                texture_index: self.find_target_index(&input.target).ok_or(FrameGraphError)?,
                                sampler: input.sampler.compile(context.device(), ()),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    let outputs = PassOutput {
                        depth: pass
                            .output
                            .depth
                            .as_ref()
                            .map(|depth| {
                                Ok((
                                    self.find_target_index(&depth.target).ok_or(FrameGraphError)?,
                                    depth.depth_operation.as_ref().map(|op| op.operation),
                                    depth.stencil_operation.as_ref().map(|op| op.operation),
                                ))
                            })
                            .transpose()?,
                        colors: pass
                            .output
                            .colors
                            .iter()
                            .map(|color| {
                                Ok((
                                    self.find_target_index(&color.target).ok_or(FrameGraphError)?,
                                    color.operation,
                                ))
                            })
                            .collect::<Result<Vec<_>, _>>()?,
                    };

                    self.passes.push(Pass {
                        name: name.clone(),
                        inputs,
                        outputs,
                    })
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
        if let Err(err) = self.compile_frame_graph(context) {
            self.descriptor = Err(err)
        }
    }

    pub fn start_frame(&mut self, surface: &Surface, context: &mut Context) -> Result<(), GameError> {
        let frame = context.create_frame(surface)?;
        self.frame_output = Some(frame);

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
        self.frame_output = None;
    }

    pub fn frame_output(&self) -> Option<&FrameOutput> {
        self.frame_output.as_ref()
    }

    pub fn frame_size(&self) -> (u32, u32) {
        self.frame_output()
            .map(|x| (x.descriptor.width, x.descriptor.height))
            .unwrap_or((0, 0))
    }

    pub fn pass_textures(&self, pass: &str) -> Result<FrameTextures<'_>, GameError> {
        if pass == "DEBUG" {
            Ok(FrameTextures {
                frame: &self.frame_output().unwrap().frame,
                textures: Vec::new(),
            })
        } else {
            if let Some(pass) = self.passes.iter().filter(|x| x.name == pass).next() {
                Ok(FrameTextures {
                    frame: &self.frame_output().unwrap().frame,
                    textures: pass
                        .inputs
                        .iter()
                        .map(|texture| (&self.targets[texture.texture_index], &texture.sampler))
                        .collect(),
                })
            } else {
                //log::warn!("No [{}] pass was found", pass);
                Err(GameError::Render(format!("No [{}] pass was found", pass)))
            }
        }
    }

    pub fn create_pass<'e, 'f: 'e>(
        &'f self,
        encoder: &'f mut wgpu::CommandEncoder,
        pass: &str,
    ) -> Result<(wgpu::RenderPass<'e>, FrameTextures<'f>), GameError> {
        if pass == "DEBUG" {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: Cow::Borrowed(&[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &self.frame_output().unwrap().frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: true,
                    },
                }]),
                depth_stencil_attachment: None,
            });

            let textures = FrameTextures {
                frame: &self.frame_output().unwrap().frame,
                textures: Vec::new(),
            };

            Ok((pass, textures))
        } else {
            if let Some(pass) = self.passes.iter().filter(|x| x.name == pass).next() {
                let color_desc = pass
                    .outputs
                    .colors
                    .iter()
                    .map(|attachement| {
                        if attachement.0 == usize::max_value() {
                            log::warn!("render target: frame");
                            // frame
                            wgpu::RenderPassColorAttachmentDescriptor {
                                attachment: &self.frame_output().unwrap().frame.view,
                                resolve_target: None,
                                ops: attachement.1,
                            }
                        } else {
                            log::warn!(
                                "render target: {} {:?}",
                                self.targets[attachement.0].name,
                                self.targets[attachement.0].render_target.size
                            );
                            wgpu::RenderPassColorAttachmentDescriptor {
                                attachment: &self.targets[attachement.0].render_target.view,
                                resolve_target: None,
                                ops: attachement.1,
                            }
                        }
                    })
                    .collect::<Vec<_>>();
                let depth_desc =
                    pass.outputs
                        .depth
                        .as_ref()
                        .map(|attachement| wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: &self.targets[attachement.0].render_target.view,
                            depth_ops: attachement.1.clone(),
                            stencil_ops: attachement.2.clone(),
                        });
                let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: Cow::Borrowed(&color_desc[..]),
                    depth_stencil_attachment: depth_desc,
                });

                // todo: create a list of texture targets
                let textures = FrameTextures {
                    frame: &self.frame_output().unwrap().frame,
                    textures: pass
                        .inputs
                        .iter()
                        .map(|texture| (&self.targets[texture.texture_index], &texture.sampler))
                        .collect(),
                };

                Ok((render_pass, textures))
            } else {
                //log::warn!("No [{}] pass was found", pass);
                Err(GameError::Render(format!("No [{}] pass was found", pass)))
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
