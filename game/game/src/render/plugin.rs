use crate::{
    app::{AppError, Plugin, PluginFuture},
    assets::AssetIO,
    render::{Context, FrameTarget, RenderError, Shader, Surface},
    World,
};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, error::Error as StdError};

pub const RENDER_PLUGIN_NAME: &str = "render";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderConfig {
    pub swap_chain_format: wgpu::TextureFormat,
    pub enable_validation: bool,
    pub wgpu_trace: Option<String>,
}

pub struct RenderPlugin {
    config: RenderConfig,
    wgpu_instance: wgpu::Instance,
    surface: Surface,
}

impl RenderPlugin {
    pub fn new(config: RenderConfig, wgpu_instance: wgpu::Instance, surface: Surface) -> RenderPlugin {
        RenderPlugin {
            config,
            wgpu_instance,
            surface,
        }
    }
}

fn into_plugin_err<E: 'static + StdError>(error: E) -> AppError {
    AppError::plugin(RENDER_PLUGIN_NAME, error)
}

impl Plugin for RenderPlugin {
    fn name() -> Cow<'static, str> {
        RENDER_PLUGIN_NAME.into()
    }

    fn init(self, world: &mut World) -> PluginFuture<()> {
        Box::pin(async move {
            let assetio = world.resources.get::<AssetIO>().map_err(into_plugin_err)?.clone();

            let context = Context::new(self.wgpu_instance, &self.surface, &self.config)
                .await
                .map_err(|err| RenderError::device_error("Failed to create context", err))
                .map_err(into_plugin_err)?;
            let device = context.device();
            let frame_target = FrameTarget::default();

            world.resources.quick_insert(self.surface).map_err(into_plugin_err)?;
            world.resources.quick_insert(context).map_err(into_plugin_err)?;
            world.resources.quick_insert(frame_target).map_err(into_plugin_err)?;

            Shader::register_resource(&mut world.resources, assetio, device).map_err(into_plugin_err)?;

            Ok(())
        })
    }

    fn deinit(world: &mut World) -> PluginFuture<()> {
        Box::pin(async move {
            let _ = world.resources.remove::<FrameTarget>();
            let _ = world.resources.remove::<Context>();
            let _ = world.resources.remove::<Surface>();

            Shader::unregister_resource(&mut world.resources);
            Ok(())
        })
    }
}

impl World {
    fn start_frame(&mut self, size: (u32, u32)) -> Result<(), AppError> {
        let mut surface = self.resources.get_mut::<Surface>().map_err(into_plugin_err)?;
        let mut context = self.resources.get_mut::<Context>().map_err(into_plugin_err)?;
        let mut frame_output = self.resources.get_mut::<FrameTarget>().map_err(into_plugin_err)?;

        surface.set_size(size);
        let (output_texture, descriptor) = context.create_frame(&surface).map_err(into_plugin_err)?;
        frame_output.set(output_texture, descriptor);
        Ok(())
    }

    fn bake_resources(&mut self, gc: bool) {
        Shader::bake_resource(&mut self.resources, gc);
    }

    fn end_frame(&mut self) -> Result<(), AppError> {
        let mut context = self.resources.get_mut::<Context>().map_err(into_plugin_err)?;
        let mut frame_output = self.resources.get_mut::<FrameTarget>().map_err(into_plugin_err)?;

        context.submit_commands();
        frame_output.present();
        Ok(())
    }
}

pub trait RenderWorld {
    fn render(&mut self, size: (u32, u32)) -> Result<(), AppError>;
}

impl RenderWorld for World {
    fn render(&mut self, size: (u32, u32)) -> Result<(), AppError> {
        self.start_frame(size)?;
        self.bake_resources(false);
        let res = self.run_stage("render");
        self.end_frame()?;
        res
    }
}
