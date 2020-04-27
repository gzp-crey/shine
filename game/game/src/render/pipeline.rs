use crate::utils::url::Url;
use crate::{
    render::{
        Context, PipelineDescriptor, ShaderDependency, ShaderStore, ShaderStoreRead, ShaderType, Vertex,
        VertexBufferLayout, VertexTypeId,
    },
    utils, wgpu, GameError,
};
use futures::future::FutureExt;
use shine_ecs::core::store::{
    CancellationToken, Data, DataLoader, DataUpdater, FromKey, Index, LoadContext, LoadListeners, ReadGuard, Store,
};
use std::fmt;
use std::ops::Range;
use std::pin::Pin;

pub struct Dependecy {
    descriptor: PipelineDescriptor,
    vertex_shader: ShaderDependency,
    fragment_shader: ShaderDependency,
}

impl Dependecy {
    fn from_descriptor(
        load_context: &LoadContext<'_, Pipeline>,
        descriptor: PipelineDescriptor,
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
                match self.descriptor.compile(context, (vs, fs)) {
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

    fn on_load(
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

            (Pipeline::Pending(listeners), PipelineLoadResponse::Descriptor(descriptor)) => {
                let dependency = Dependecy::from_descriptor(&load_context, descriptor, shaders);
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
    pub fn new<V: Vertex>(name: &str) -> PipelineKey {
        PipelineKey {
            name: name.to_owned(),
            vertex_type: <V as Vertex>::id(),
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
    Descriptor(PipelineDescriptor),
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
        let vertex_attributes = VertexBufferLayout::from_id(&pipeline_key.vertex_type);
        log::trace!("Vertex attributes: {:#?}", vertex_attributes);

        let url = match self.base_url.join(&source_id) {
            Err(err) => {
                let err = format!("Invalid pipeline url ({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResponse::Error(err));
            }
            Ok(url) => url,
        };

        let data = match utils::assets::download_string(&url).await {
            Err(err) => {
                let err = format!("Failed to get pipeline({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResponse::Error(err));
            }
            Ok(data) => data,
        };
        log::trace!("pipeline [{}]: {}", source_id, data);

        let descriptor: PipelineDescriptor = match serde_json::from_str(&data) {
            Err(err) => {
                let err = format!("Failed to parse pipeline({}): {:?}", source_id, err);
                log::warn!("{}", err);
                return Some(PipelineLoadResponse::Error(err));
            }
            Ok(descriptor) => descriptor,
        };

        if let Err(err) = descriptor.vertex_stage.check_vertex_layout(&vertex_attributes) {
            let err = format!(
                "Pipeline and vertex layouts are not matching [{}]: {:?}",
                source_id, err
            );
            log::warn!("{}", err);
            return Some(PipelineLoadResponse::Error(err));
        }

        Some(PipelineLoadResponse::Descriptor(descriptor))
    }
}

impl DataLoader<Pipeline> for PipelineLoader {
    fn load<'a>(
        &'a mut self,
        pipeline_key: PipelineKey,
        cancellation_token: CancellationToken<Pipeline>,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<PipelineLoadResponse>> + Send + 'a>> {
        self.load_from_url(cancellation_token, pipeline_key).boxed()
    }
}

impl<'a> DataUpdater<'a, Pipeline> for (&Context, &ShaderStore) {
    fn update<'u>(
        &mut self,
        load_context: LoadContext<'u, Pipeline>,
        data: &mut Pipeline,
        load_response: PipelineLoadResponse,
    ) -> Option<PipelineLoadRequest> {
        data.on_load(load_context, self.0, &mut self.1.read(), load_response)
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

    #[inline]
    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.render_pass.draw(vertices, instances)
    }

    #[inline]
    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass.draw_indexed(indices, base_vertex, instances);
    }
}

pub type PipelineStore = Store<Pipeline>;
pub type PipelineStoreRead<'a> = ReadGuard<'a, Pipeline>;
pub type PipelineIndex = Index<Pipeline>;
