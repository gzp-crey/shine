use crate::ecs::{
    resources::Resources,
    scheduler::{IntoSystem, System},
    ECSError,
};
use std::sync::{Arc, Mutex};

/// A collection of systems.
/// Schedules are essentially the "execution plan" for an App's systems.
/// They are run on a given [World] and [Resources] reference.
#[derive(Default)]
pub struct Schedule {
    pub(crate) systems: Vec<Arc<Mutex<Box<dyn System>>>>,
}

impl Schedule {
    pub fn schedule<R, Func: IntoSystem<R>>(&mut self, func: Func) -> Result<(), ECSError> {
        let system = func.into_system()?;
        self.systems.push(Arc::new(Mutex::new(system)));
        Ok(())
    }

    pub fn run(&mut self, resources: &Resources) -> Result<(), ECSError> {
        for system in self.systems.iter() {
            let mut system = system.lock().unwrap();
            system.run(resources)?;
        }
        Ok(())
    }
}
