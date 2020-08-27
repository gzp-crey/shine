use crate::{
    assets::FrameGraphDescriptor,
    render::{Context, Frame, RenderResources},
    GameView,
};

#[derive(Debug)]
pub enum RenderError {
    Driver(String),
    Output,
    GraphNotReady,
    GraphError,
    GraphInconsistency,
    MissingFramePass(String),
}

pub trait RenderPlugin {
    fn add_render_plugin(&mut self, context: Context) -> Result<(), RenderError>;
    //fn remove_render_plugin(&mut self) -> Result<(), RenderError>;
    //fn load_frame_graph(&mut self, graph_id: String);
    fn set_frame_graph(&mut self, graph: FrameGraphDescriptor) -> Result<(), RenderError>;
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
        self.resources.insert(RenderResources::new(&self.assetio));

        Ok(())
    }

    fn set_frame_graph(&mut self, graph: FrameGraphDescriptor) -> Result<(), RenderError> {
        let mut frame = self.resources.get_mut::<Frame>().unwrap();
        frame.set_frame_graph(graph)
    }

    fn render(&mut self, size: (u32, u32)) -> Result<(), RenderError> {
        self.start_frame(size)?;
        self.run_logic("render");
        self.end_frame();
        Ok(())
    }
}
