use crate::{
    assets::FrameGraphDescriptor,
    render::{frame_graph::FrameGraphError, Context, Frame, Model, Pipeline, Shader, Texture},
    GameView,
};
use shine_ecs::core::store;

#[derive(Debug)]
pub enum RenderError {
    Driver(String),
    Output,
    GraphNotReady,
    GraphError,
    MissingPass(String),
}

impl From<FrameGraphError> for RenderError {
    fn from(err: FrameGraphError) -> RenderError {
        RenderError::GraphError
    }
}

pub trait RenderPlugin {
    fn add_render_plugin(&mut self, context: Context) -> Result<(), RenderError>;
    //fn remove_render_plugin(&mut self) -> Result<(), RenderError>;
    fn set_frame_graph(&mut self, graph_id: Option<String>);
    fn render(&mut self, size: (u32, u32)) -> Result<(), RenderError>;
}

impl GameView {
    fn start_frame(&mut self, size: (u32, u32)) -> Result<(), RenderError> {
        let surface = &mut self.surface;
        surface.set_size(size);

        let mut context = self.resources.get_mut::<Context>().unwrap();
        let mut frame = self.resources.get_mut::<Frame>().unwrap();
        frame.start_frame(surface, &mut context)?;

        Ok(())
    }

    fn end_frame(&mut self) {
        let context = self.resources.get_mut::<Context>().unwrap();
        let mut frame = self.resources.get_mut::<Frame>().unwrap();
        frame.end_frame(context.queue());
    }
}

impl RenderPlugin for GameView {
    fn add_render_plugin(&mut self, context: Context) -> Result<(), RenderError> {
        log::info!("Adding render plugin");

        self.resources.insert(context);
        self.resources.insert(Frame::new());

        self.resources
            .insert(store::async_load::<Shader, _>(16, self.assetio.clone()));
        self.resources
            .insert(store::async_load::<Texture, _>(16, self.assetio.clone()));
        self.resources
            .insert(store::async_load::<Pipeline, _>(16, self.assetio.clone()));
        self.resources
            .insert(store::async_load::<Model, _>(16, self.assetio.clone()));

        Ok(())
    }

    //fn remove_render_plugin(&mut self) -> Result<(), RenderError> {}

    fn set_frame_graph(&mut self, graph_id: Option<String>) {
        /*if let Some(mut frame) = self.resources.get_mut::<Frame>() {
            if let Some(graph_id) = graph_id {
                frame.load_graph(self.assetio.clone(), graph_id)
            } else {
                frame.set_graph(Ok(FrameGraphDescriptor::single_pass()));
            }
        }*/
    }

    fn render(&mut self, size: (u32, u32)) -> Result<(), RenderError> {
        self.start_frame(size)?;
        self.run_logic("render");
        self.end_frame();
        Ok(())
    }
}
