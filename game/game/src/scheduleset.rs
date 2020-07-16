use crate::GameError;
use shine_ecs::legion::{
    systems::{resource::Resources, schedule::Schedule},
    world::World,
};
use std::collections::HashMap;

pub struct ScheduleSet {
    logics: HashMap<String, Schedule>,
}

impl ScheduleSet {
    pub fn new() -> ScheduleSet {
        ScheduleSet { logics: HashMap::new() }
    }

    pub fn execute(&mut self, logic: &str, world: &mut World, resources: &mut Resources) {
        if let Some(schedule) = self.logics.get_mut(logic) {
            schedule.execute(world, resources);
        } else {
            //log::warn!("logic [{}] not found", logic);
        }
    }

    pub fn insert(&mut self, name: &str, logic: Schedule) -> Result<(), GameError> {
        log::info!("Registering schedule {}", name);
        use std::collections::hash_map::Entry;
        match self.logics.entry(name.to_owned()) {
            entry @ Entry::Vacant(_) => {
                entry.or_insert(logic);
                Ok(())
            }
            Entry::Occupied(_) => Err(GameError::Setup(format!("Logic {} already registered", name))),
        }
    }

    pub fn remove(&mut self, name: &str) {
        log::info!("Unregistering schedule {}", name);
        let _ = self.logics.remove(&name.to_owned());
    }
}

impl Default for ScheduleSet {
    fn default() -> Self {
        Self::new()
    }
}
