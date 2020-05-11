use crate::utils::url::Url;
use crate::{
    render::{
        Context, IntoVertexTypeId, PipelineDescriptor, ShaderDependency, ShaderStore, ShaderStoreRead, ShaderType,
        VertexBufferLayout, VertexTypeId,
    },
    utils, wgpu, GameError,
};
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, DataUpdater, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;

pub struct Dependecy {
    descriptor: Box<PipelineDescriptor>,
    vertex_layouts: Vec<VertexBufferLayout>,
    vertex_shader: ShaderDependency,
    fragment_shader: ShaderDependency,
}

impl Dependecy {
    fn from_descriptor(
        load_context: &LoadContext<'_, Pipeline>,
        descriptor: Box<PipelineDescriptor>,
        vertex_layouts: Vec<VertexBufferLayout>,
        shaders: &mut ShaderStoreRead<'_>,
    ) -> Dependecy {
        let vertex_shader = ShaderDependency::new(
            shaders,
            &descriptor.vertex_stage.shader,
            ShaderType::Vertex,
            load_context,
            PipelineLoadResponse::ShaderReady(ShaderType::Vertex),
        );
        let fragment_shader = ShaderDependency::new(
            shaders,
            &descriptor.fragment_stage.shader,
            ShaderType::Fragment,
            load_context,
            PipelineLoadResponse::ShaderReady(ShaderType::Fragment),
        );

        Dependecy {
            descriptor,
            vertex_layouts,
            vertex_shader,
            fragment_shader,
        }
    }

    fn with_updated_shader_dependency(self, shader_type: ShaderType, shaders: &mut ShaderStoreRead<'_>) -> Self {
        match shader_type {
            ShaderType::Vertex => Dependecy {
                vertex_shader: self.vertex_shader.update(shaders),
                ..self
            },
            ShaderType::Fragment => Dependecy {
                fragment_shader: self.fragment_shader.update(shaders),
                ..self
            },
            _ => unreachable!(),
        }
    }

    fn into_pipeline(
        self,
        load_context: &LoadContext<'_, Pipeline>,
        context: &Context,
        shaders: &mut ShaderStoreRead<'_>,
        listeners: LoadListeners,
    ) -> Pipeline {
        match (&self.vertex_shader, &self.fragment_shader) {
            (ShaderDependency::Failed(err), _) => {
                listeners.notify_all();
                Pipeline::Error(format!("Vertex shader error: {}", err))
            }
            (_, ShaderDependency::Failed(err)) => {
                listeners.notify_all();
                Pipeline::Error(format!("Fragment shader error: {}", err))
            }
            (ShaderDependency::Pending(_, _), _) => Pipeline::WaitingDependency(self, listeners),
            (_, ShaderDependency::Pending(_, _)) => Pipeline::WaitingDependency(self, listeners),
            (ShaderDependency::Completed(vs), ShaderDependency::Completed(fs)) => {
                log::debug!("Pipeline compilation completed [{:?}]", load_context);
                listeners.notify_all();
                let vs = shaders.at(&vs).shadere_module().unwrap();
                let fs = shaders.at(&fs).shadere_module().unwrap();
                match self.descriptor.compile(context, &self.vertex_layouts, (vs, fs)) {
                    Ok(pipeline) => Pipeline::Compiled(pipeline),
                    Err(err) => Pipeline::Error(err),
                }
            }
        }
    }
}

pub enum Pipeline {
    Pending(LoadListeners),
    WaitingDependency(Dependecy, LoadListeners),
    Compiled(wgpu::RenderPipeline),
    Error(String),
    None,
}

impl Pipeline {
    pub fn bind<'a: 'pass, 'pass>(
        &'a self,
        encoder: &'a mut wgpu::CommandEncoder,
        pass_descriptor: &wgpu::RenderPassDescriptor<'pass, 'pass>,
    ) -> Option<BoundPipeline<'a, 'pass>> {
        match self {
            Pipeline::Compiled(ref pipeline) => {
                let mut b = BoundPipeline {
                    pipeline,
                    render_pass: encoder.begin_render_pass(pass_descriptor),
                };
                b.bind_pipeline();
                Some(b)
            }
            _ => None,
        }
    }

    fn on_update(
        &mut self,
        load_context: LoadContext<'_, Pipeline>,
        context: &Context,
        shaders: &mut ShaderStoreRead<'_>,
        load_response: PipelineLoadResponse,
    ) -> Option<PipelineKey> {
        *self = match (std::mem::replace(self, Pipeline::None), load_response) {
            (Pipeline::Pending(listeners), PipelineLoadResponse::Error(err)) => {
                log::debug!("Pipeline compilation failed [{:?}]: {}", load_context, err);
                listeners.notify_all();
                Pipeline::Error(err)
            }

            (Pipeline::Pending(listeners), PipelineLoadResponse::Descriptor(descriptor, vertex_layouts)) => {
                let dependency = Dependecy::from_descriptor(&load_context, descriptor, vertex_layouts, shaders);
                dependency.into_pipeline(&load_context, context, shaders, listeners)
            }

            (Pipeline::WaitingDependency(dependency, listeners), PipelineLoadResponse::ShaderReady(shader_type)) => {
                dependency
                    .with_updated_shader_dependency(shader_type, shaders)
                    .into_pipeline(&load_context, context, shaders, listeners)
            }

            (err @ Pipeline::Error(_), PipelineLoadResponse::ShaderReady(_)) => err,

            (Pipeline::WaitingDependency(_, _), _) => unreachable!(),
            (Pipeline::Pending(_), _) => unreachable!(),
            (Pipeline::Compiled(_), _) => unreachable!(),
            (Pipeline::Error(_), _) => unreachable!(),
            (Pipeline::None, _) => unreachable!(),
        };

        None
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct PipelineKey {
    pub name: String,
    pub vertex_type: VertexTypeId,
}

impl PipelineKey {
    pub fn new<V: IntoVertexTypeId>(name: &str) -> PipelineKey {
        PipelineKey {
            name: name.to_owned(),
            vertex_type: <V as IntoVertexTypeId>::into_id(),
        }
    }
}

impl fmt::Debug for PipelineKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("").field(&self.name).field(&self.vertex_type).finish()
    }
}

impl Data for Pipeline {
    type Key = PipelineKey;
    type LoadRequest = PipelineLoadRequest;
    type LoadResponse = PipelineLoadResponse;
}

impl FromKey for Pipeline {
    fn from_key(key: &PipelineKey) -> (Self, Option<PipelineKey>) {
        (Pipeline::Pending(LoadListeners::new()), Some(key.to_owned()))
    }
}

pub type PipelineLoadRequest = PipelineKey;

pub enum PipelineLoadResponse {
    Error(String),
    Descriptor(Box<PipelineDescriptor>, Vec<VertexBufferLayout>),
    ShaderReady(ShaderType),
}

pub struct PipelineLoader {
    base_url: Url,
}

impl PipelineLoader {
    pub fn new(base_url: &str) -> Result<PipelineLoader, GameError> {
        let base_url = Url::parse(base_url)
            .map_err(|err| GameError::Config(format!("Failed to parse base url for pipeline: {:?}", err)))?;

        Ok(PipelineLoader { base_url })
    }

    async fn load_from_url(
        &mut self,
        cancellation_token: CancellationToken<Pipeline>,
        pipeline_key: PipelineKey,
    ) -> Option<PipelineLoadResponse> {
        if cancellation_token.is_canceled() {
            return None;
        }

        let source_id = &pipeline_key.name;
        let vertex_layouts = pipeline_key.vertex_type.to_layout();
        log::trace!("Vertex attributes: {:#?}", vertex_layouts);

        let url = match self.base_url.join(&source_id) {
            Err(err) => {
                let err = format!("Invalid pipeline url ({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResponse::Error(err));
            }
            Ok(url) => url,
        };

        let data = match utils::assets::download_binary(&url).await {
            Err(err) => {
                let err = format!("Failed to get pipeline({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResponse::Error(err));
            }
            Ok(data) => data,
        };

        let descriptor: PipelineDescriptor = match bincode::deserialize(&data) {
            Err(err) => {
                let err = format!("Failed to parse pipeline({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResponse::Error(err));
            }
            Ok(descriptor) => descriptor,
        };
        log::trace!("pipeline [{}]: {:#?}", source_id, descriptor);

        if let Err(err) = descriptor.vertex_stage.check_vertex_layouts(&vertex_layouts) {
            let err = format!(
                "Pipeline and vertex layouts are not compatible [{}]: {:?}",
                source_id, err
            );
            log::warn!("{}", err);
            return Some(PipelineLoadResponse::Error(err));
        }

        Some(PipelineLoadResponse::Descriptor(Box::new(descriptor), vertex_layouts))
    }
}

impl DataLoader<Pipeline> for PipelineLoader {
    fn load<'a>(
        &'a mut self,
        pipeline_key: PipelineKey,
        cancellation_token: CancellationToken<Pipeline>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<PipelineLoadResponse>>>> {
        Box::pin(self.load_from_url(cancellation_token, pipeline_key))
    }
}

impl<'a> DataUpdater<'a, Pipeline> for (&Context, &ShaderStore) {
    fn update<'u>(
        &mut self,
        load_context: LoadContext<'u, Pipeline>,
        data: &mut Pipeline,
        load_response: PipelineLoadResponse,
    ) -> Option<PipelineLoadRequest> {
        data.on_update(load_context, self.0, &mut self.1.read(), load_response)
    }
}

pub struct BoundPipeline<'a: 'pass, 'pass> {
    pipeline: &'a wgpu::RenderPipeline,
    render_pass: wgpu::RenderPass<'pass>,
}

impl<'a: 'pass, 'pass> BoundPipeline<'a, 'pass> {
    #[inline]
    fn bind_pipeline(&mut self) {
        self.render_pass.set_pipeline(self.pipeline);
    }
}

impl<'a: 'pass, 'pass> Deref for BoundPipeline<'a, 'pass> {
    type Target = wgpu::RenderPass<'pass>;
    fn deref(&self) -> &Self::Target {
        &self.render_pass
    }
}

impl<'a: 'pass, 'pass> DerefMut for BoundPipeline<'a, 'pass> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.render_pass
    }
}

pub type PipelineStore = Store<Pipeline>;
pub type PipelineStoreRead<'a> = ReadGuard<'a, Pipeline>;
pub type PipelineIndex = Index<Pipeline>;

pub mod systems {
    use super::*;
    use shine_ecs::legion::systems::{schedule::Schedulable, SystemBuilder};

    pub fn update_pipeline() -> Box<dyn Schedulable> {
        SystemBuilder::new("update_pipeline")
            .read_resource::<Context>()
            .read_resource::<ShaderStore>()
            .write_resource::<PipelineStore>()
            .build(move |_, _, (context, shaders, pipeline), _| {
                //log::info!("pipeline");
                let mut pipeline = pipeline.write();
                let context: &Context = &*context;
                let shaders: &ShaderStore = &*shaders;
                //shaders.drain_unused();
                pipeline.update(&mut (context, shaders));
                pipeline.finalize_requests();
            })
    }
}
