use crate::{
    assets::{AssetError, AssetIO, FrameGraphDescriptor},
    render::{Context, Frame, RenderResources, Surface},
    GameView,
};
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};

#[derive(Debug)]
pub enum RenderError {
    MissingPlugin(String),
    Driver(String),
    Asset(AssetError),
    Output,
    GraphNotReady,
    GraphError,
    GraphInconsistency,
    MissingFramePass(String),
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
    fn add_render_plugin<'a>(
        &'a mut self,
        config: RenderConfig,
        wgpu_instance: wgpu::Instance,
        surface: Surface,
    ) -> RenderFuture<'a, Result<(), RenderError>>;

    /// Remove render plugin from the world
    fn remove_render_plugin<'a>(&'a mut self) -> RenderFuture<'a, Result<(), RenderError>>;

    //fn load_frame_graph(&mut self, graph_id: String);
    fn set_frame_graph(&mut self, graph: FrameGraphDescriptor) -> Result<(), RenderError>;
    fn render(&mut self, size: (u32, u32)) -> Result<(), RenderError>;
}

impl GameView {
    fn start_frame(&mut self, size: (u32, u32)) -> Result<(), RenderError> {
        let mut surface = self.resources.get_mut::<Surface>(None).unwrap();
        let mut context = self.resources.get_mut::<Context>(None).unwrap();
        let mut frame = self.resources.get_mut::<Frame>(None).unwrap();

        surface.set_size(size);
        frame.start_frame(&surface, &mut context)?;

        Ok(())
    }

    fn end_frame(&mut self) {
        let context = self.resources.get::<Context>(None).unwrap();
        let mut frame = self.resources.get_mut::<Frame>(None).unwrap();
        frame.end_frame(context.queue());
    }
}

impl RenderPlugin for GameView {
    fn add_render_plugin<'a>(
        &'a mut self,
        config: RenderConfig,
        wgpu_instance: wgpu::Instance,
        surface: Surface,
    ) -> RenderFuture<'a, Result<(), RenderError>> {
        Box::pin(async move {
            log::info!("Adding render plugin");
            let assetio = self
                .resources
                .get::<AssetIO>(None)
                .ok_or(RenderError::MissingPlugin("AssetIO".to_owned()))?
                .clone();

            let context = Context::new(wgpu_instance, &surface, &config)
                .await
                .map_err(|err| RenderError::Driver(format!("Failed to create context: {:?}", err)))?;

            self.resources.insert(None, surface);
            self.resources.insert(None, context);
            self.resources.insert(None, Frame::new());
            self.resources.insert(None, RenderResources::new(&assetio));

            Ok(())
        })
    }

    fn remove_render_plugin<'a>(&'a mut self) -> RenderFuture<'a, Result<(), RenderError>> {
        Box::pin(async move {
            let _ = self.resources.remove::<RenderResources>(None);
            let _ = self.resources.remove::<Frame>(None);
            let _ = self.resources.remove::<Context>(None);
            let _ = self.resources.remove::<Surface>(None);

            Ok(())
        })
    }

    fn set_frame_graph(&mut self, graph: FrameGraphDescriptor) -> Result<(), RenderError> {
        let mut frame = self.resources.get_mut::<Frame>(None).unwrap();
        frame.set_frame_graph(graph)
    }

    fn render(&mut self, size: (u32, u32)) -> Result<(), RenderError> {
        {
            let context = self.resources.get::<Context>(None).unwrap();
            let mut resources = self.resources.get_mut::<RenderResources>(None).unwrap();
            resources.update(&context);
        }

        self.start_frame(size)?;
        //self.run_logic("render");
        self.end_frame();
        Ok(())
    }
}
