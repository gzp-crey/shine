use crate::resources::Resources;
use crate::scheduler::{IntoSystem, System};
use std::sync::{Arc, Mutex};

/// A collection of systems.
/// Schedules are essentially the "execution plan" for an App's systems.
/// They are run on a given [World] and [Resources] reference.
#[derive(Default)]
pub struct Schedule {
    pub(crate) systems: Vec<Arc<Mutex<Box<dyn System>>>>,
}

impl Schedule {
    pub fn schedule<State, R, Func: IntoSystem<State, R>>(&mut self, func: Func) {
        let system = func.into_system();
        self.systems.push(Arc::new(Mutex::new(system)));
    }

    pub fn run(&mut self, resources: &Resources) {
        for system in self.systems.iter() {
            let mut system = system.lock().unwrap();
            system.run(resources);
        }
    }
}
