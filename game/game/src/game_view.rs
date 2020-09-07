use shine_ecs::resources::Resources;

pub struct GameView {
    pub resources: Resources,
}

impl GameView {
    pub fn new() -> GameView {
        GameView {
            resources: Resources::default(),
        }
    }

    /*pub fn run_logic(&mut self, logic: &str) {
        let world = &mut self.world;
        let resources = &mut self.resources;
        self.schedules.execute(logic, world, resources);
    }*/

    /*pub fn refresh(&mut self, size: (u32, u32)) -> Result<(), GameError> {
        self.run_logic("prepare_update");
        self.run_logic("update");
        //log::trace!("render size: {:?}", size);
        self.render(size)?;
        Ok(())
    }*/

    /*pub fn gc(&mut self) {
        self.run_logic("gc");
    }*/
}
