use crate::{
    assets::{AssetIO, CookedShader, Url},
    render::{Compile, CompiledShader},
};
use serde::{Deserialize, Serialize};
use shine_ecs::{
    core::observer::ObserveDispatcher,
    resources::{ResourceHandle, ResourceId, ResourceLoadRequester, ResourceLoadResponder, ResourceLoader, Resources},
    ECSError,
};
use std::sync::Arc;

pub struct ShaderError;

/// Unique key for a shader
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ShaderKey(String);

impl ShaderKey {
    pub fn new<S: ToString>(id: S) -> Self {
        Self(id.to_string())
    }
}

#[derive(Debug)]
pub enum ShaderEvent {
    Loaded,
}

pub struct Shader {
    id: String,
    shader: Result<Option<CompiledShader>, ShaderError>,
    dispatcher: ObserveDispatcher<ShaderEvent>,
}

impl Shader {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn dispatcher(&self) -> &ObserveDispatcher<ShaderEvent> {
        &self.dispatcher
    }

    pub fn shader(&self) -> Result<Option<&CompiledShader>, ShaderError> {
        match &self.shader {
            Err(_) => Err(ShaderError),
            Ok(None) => Ok(None),
            Ok(Some(shader)) => Ok(Some(shader)),
        }
    }

    pub fn shader_module(&self) -> Option<&CompiledShader> {
        self.shader.as_ref().map(|u| u.as_ref()).unwrap_or(None)
    }
}

struct LoadRequest(String);

enum LoadResponse {
    Compiled(CompiledShader),
    Error(ShaderError),
}

/// Implement functions to make it a resource
impl Shader {
    fn build(
        context: &ResourceLoadRequester<Self, LoadRequest>,
        handle: ResourceHandle<Self>,
        id: &ResourceId,
    ) -> Self {
        log::trace!("Creating [{:?}]", id);
        if let Ok(ShaderKey(id)) = id.to_object::<ShaderKey>() {
            context.send_request(handle, LoadRequest(id.clone()));
            Shader {
                id,
                shader: Ok(None),
                dispatcher: Default::default(),
            }
        } else {
            Shader {
                id: Default::default(),
                shader: Err(ShaderError),
                dispatcher: Default::default(),
            }
        }
    }

    async fn load_and_compile(
        (io, device): &(AssetIO, Arc<wgpu::Device>),
        handle: &ResourceHandle<Self>,
        shader_id: String,
    ) -> Result<CompiledShader, ShaderError> {
        log::debug!("[{:?}] Loading shader...", shader_id);

        let url = Url::parse(&shader_id).map_err(|_| ShaderError)?;
        let data = io.download_binary(&url).await.map_err(|_| ShaderError)?;

        log::debug!("[{:?}] Extracting shader...", shader_id);
        handle.check_liveness().map_err(|_| ShaderError)?;
        let cooked_shader: CookedShader = bincode::deserialize_from(&*data).map_err(|_| ShaderError)?;

        log::debug!("[{:?}] Compiling shader...", shader_id);
        handle.check_liveness().map_err(|_| ShaderError)?;
        let compiled_shader = cooked_shader.compile(&*device);

        log::debug!("[{:?}] Shader loaded", shader_id);
        Ok(compiled_shader)
    }

    async fn on_load(
        ctx: &(AssetIO, Arc<wgpu::Device>),
        responder: &ResourceLoadResponder<Shader, LoadResponse>,
        handle: ResourceHandle<Self>,
        request: LoadRequest,
    ) {
        let LoadRequest(shader_id) = request;
        let response = match Self::load_and_compile(ctx, &handle, shader_id).await {
            Ok(shader) => LoadResponse::Compiled(shader),
            Err(err) => LoadResponse::Error(err),
        };
        responder.send_response(handle, response);
    }

    fn on_load_response(
        this: &mut Self,
        _requester: &ResourceLoadRequester<Self, LoadRequest>,
        _handle: &ResourceHandle<Self>,
        response: LoadResponse,
    ) {
        log::debug!("[{:?}] Load completed", this.id);
        match response {
            LoadResponse::Compiled(shader) => this.shader = Ok(Some(shader)),
            LoadResponse::Error(err) => this.shader = Err(err),
        };
        this.dispatcher.notify_all(ShaderEvent::Loaded);
    }

    pub fn register_resource(
        resources: &mut Resources,
        io: AssetIO,
        device: Arc<wgpu::Device>,
    ) -> Result<(), ECSError> {
        resources.register(ResourceLoader::new(
            Shader::build,
            (io, device),
            Shader::on_load,
            Shader::on_load_response,
        ))
    }

    pub fn unregister_resource(resources: &mut Resources) {
        resources.unregister::<Shader>();
    }

    pub fn bake_resource(resources: &mut Resources, gc: bool) {
        resources.bake::<Shader>(gc);
    }
}

pub type ShaderHandle = ResourceHandle<Shader>;
