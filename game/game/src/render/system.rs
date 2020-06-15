use crate::render::{Context, Frame, ModelLoader, PipelineLoader, ShaderLoader, TextureLoader};
use crate::{GameError, GameView};

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

        self.register_store(ShaderLoader::new(self.assetio.clone()), 16);
        self.register_store(PipelineLoader::new(self.assetio.clone()), 16);
        self.register_store(ModelLoader::new(self.assetio.clone()), 16);
        self.register_store(TextureLoader::new(self.assetio.clone()), 16);

        Ok(())
    }

    fn render(&mut self, size: (u32, u32)) -> Result<(), GameError> {
        self.start_frame(size)?;
        self.run_logic("render");
        self.end_frame()
    }
}
