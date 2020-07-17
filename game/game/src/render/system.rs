use crate::render::{Context, Frame, /*FrameGraphLoader,*/ Model, Pipeline, Shader, Texture};
use crate::{GameError, GameView};
use shine_ecs::core::store;

pub trait RenderSystem {
    fn add_render_system(&mut self, context: Context) -> Result<(), GameError>;
    fn render(&mut self, size: (u32, u32)) -> Result<(), GameError>;
}

impl GameView {
    fn start_frame(&mut self, size: (u32, u32)) -> Result<(), GameError> {
        let surface = &mut self.surface;
        surface.set_size(size);
        let mut context = self.resources.get_mut::<Context>().unwrap();
        let mut frame = self.resources.get_mut::<Frame>().unwrap();
        frame.start(context.create_frame(surface)?);
        Ok(())
    }

    fn end_frame(&mut self) -> Result<(), GameError> {
        let context = self.resources.get_mut::<Context>().unwrap();
        let mut frame = self.resources.get_mut::<Frame>().unwrap();
        frame.end(context.queue());
        Ok(())
    }
}

impl RenderSystem for GameView {
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
        //self.resources.insert(store::async_load::<FrameGraph>(16, assetio.clone()));

        Ok(())
    }

    fn render(&mut self, size: (u32, u32)) -> Result<(), GameError> {
        self.start_frame(size)?;
        self.run_logic("render");
        self.end_frame()
    }
}
