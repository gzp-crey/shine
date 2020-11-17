use shine_ecs::resources::Resources;

#[derive(Default)]
pub struct World {
    pub resources: Resources,
    //schedules: HashMap<String, Schedule>,
}

impl World {
    /*pub fn add_stage(&mut self, stage: &str, schedule: Schedule) {
        let _ = self.schedules.insert(stage.into(), schedule);
    }

    pub fn remove_stage(&mut self, stage: &str) {
        let _ = self.schedules.remove(stage);
    }

    pub fn clear_stages(&mut self) {
        self.schedules.clear();
    }

    pub fn run_stage(&mut self, stage: &str) {
        if let Some(stage) = self.schedules.get_mut(stage) {
            stage.run(&self.resources);
        }
    }*/
}
