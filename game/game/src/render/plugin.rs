use crate::{
    assets::FrameGraphDescriptor,
    render::{Context, Frame, FrameStartError, Model, Pipeline, Shader, Texture},
    GameError, GameView,
};
use shine_ecs::core::store;

pub trait RenderPlugin {
    fn add_render_system(&mut self, context: Context) -> Result<(), GameError>;
    fn set_frame_graph(&mut self, graph_id: Option<String>);
    fn render(&mut self, size: (u32, u32)) -> Result<(), GameError>;
}

impl GameView {
    fn start_frame(&mut self, size: (u32, u32)) -> Result<(), FrameStartError> {
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
    fn add_render_system(&mut self, context: Context) -> Result<(), GameError> {
        log::info!("adding render system to the world");

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

    fn set_frame_graph(&mut self, graph_id: Option<String>) {
        let context = self.resources.get_mut::<Context>().unwrap();
        if let Some(mut frame) = self.resources.get_mut::<Frame>() {
            if let Some(graph_id) = graph_id {
                frame.load_graph(&*context, self.assetio.clone(), graph_id)
            } else {
                frame.set_graph(&*context, Ok(FrameGraphDescriptor::single_pass()));
            }
        }
    }

    fn render(&mut self, size: (u32, u32)) -> Result<(), GameError> {
        self.start_frame(size)
            .map_err(|err| GameError::Render(format!("Start frame failed: {:?}", err)))?;
        self.run_logic("render");
        self.end_frame();
        Ok(())
    }
}
