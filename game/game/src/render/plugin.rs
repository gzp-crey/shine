use crate::{
    assets::{AssetError, AssetIO},
    render::{Context, FrameTarget, RenderResources, Surface},
    World, WorldError,
};
use serde::{Deserialize, Serialize};
use shine_ecs::core::DisplayError;
use std::{error::Error as StdError, future::Future, pin::Pin};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error(transparent)]
    World(#[from] WorldError),

    #[error(transparent)]
    Asset(#[from] AssetError),

    #[error("{message}")]
    Device {
        message: String,
        source: Box<dyn 'static + StdError + Send + Sync>,
    },

    #[error("Render graph missing frame pass {0}")]
    MissingFramePass(String),
}

impl RenderError {
    pub fn device_error_str<S: ToString>(err: S) -> Self {
        RenderError::Device {
            message: "Device error".to_owned(),
            source: Box::new(DisplayError(err.to_string())),
        }
    }

    pub fn device_error<S: ToString, E: 'static + StdError + Send + Sync>(message: S, err: E) -> Self {
        RenderError::Device {
            message: message.to_string(),
            source: Box::new(err),
        }
    }
}

pub type RenderFuture<'a, R> = Pin<Box<dyn Future<Output = R> + 'a>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderConfig {
    pub swap_chain_format: wgpu::TextureFormat,
    pub enable_validation: bool,
    pub wgpu_trace: Option<String>,
}

pub trait RenderPlugin {
    /// Add render plugin to the world
    fn add_render_plugin(
        &mut self,
        config: RenderConfig,
        wgpu_instance: wgpu::Instance,
        surface: Surface,
    ) -> RenderFuture<'_, Result<(), RenderError>>;

    /// Remove render plugin from the world
    fn remove_render_plugin(&mut self) -> RenderFuture<'_, Result<(), RenderError>>;

    /// Render frame using the registered "render" schedule.
    fn render(&mut self, size: (u32, u32)) -> Result<(), RenderError>;
}

impl World {
    fn start_frame(&mut self, size: (u32, u32)) -> Result<(), RenderError> {
        let mut surface = self.plugin_resource_mut::<Surface>("render")?;
        let mut context = self.plugin_resource_mut::<Context>("render")?;
        let mut frame_output = self.plugin_resource_mut::<FrameTarget>("render")?;
        let mut resources = self.plugin_resource_mut::<RenderResources>("render")?;

        surface.set_size(size);
        let (output_texture, descriptor) = context.create_frame(&surface)?;
        frame_output.set(output_texture, descriptor);
        resources.update(&context);

        Ok(())
    }

    fn end_frame(&mut self) -> Result<(), RenderError> {
        let mut context = self.plugin_resource_mut::<Context>("render")?;
        let mut frame_output = self.plugin_resource_mut::<FrameTarget>("render")?;
        context.submit_commands();
        frame_output.present();
        Ok(())
    }
}

impl RenderPlugin for World {
    fn add_render_plugin(
        &mut self,
        config: RenderConfig,
        wgpu_instance: wgpu::Instance,
        surface: Surface,
    ) -> RenderFuture<'_, Result<(), RenderError>> {
        Box::pin(async move {
            log::info!("Adding render plugin");

            let assetio = self.plugin_resource::<AssetIO>("asset")?.clone();
            let context = Context::new(wgpu_instance, &surface, &config)
                .await
                .map_err(|err| RenderError::device_error("Failed to create context", err))?;

            self.resources.insert(surface);
            self.resources.insert(context);
            self.resources.insert(FrameTarget::default());
            self.resources.insert(RenderResources::new(&assetio));

            Ok(())
        })
    }

    fn remove_render_plugin(&mut self) -> RenderFuture<'_, Result<(), RenderError>> {
        Box::pin(async move {
            let _ = self.resources.remove::<RenderResources>();
            let _ = self.resources.remove::<FrameTarget>();
            let _ = self.resources.remove::<Context>();
            let _ = self.resources.remove::<Surface>();

            Ok(())
        })
    }

    fn render(&mut self, size: (u32, u32)) -> Result<(), RenderError> {
        self.start_frame(size)?;
        self.run_stage("render");
        self.end_frame()?;
        Ok(())
    }
}
