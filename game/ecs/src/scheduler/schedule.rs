use crate::{
    resources::Resources,
    scheduler::{IntoSystem, SystemGroup},
    ECSError,
};

/// A collection of systems.
/// Schedules are essentially the "execution plan" for an App's systems.
/// They are run on a given [World] and [Resources] reference.
#[derive(Default)]
pub struct Schedule {
    system_group: SystemGroup,
}

impl Schedule {
    pub fn schedule<R, Func: IntoSystem<R>>(&mut self, func: Func) -> Result<(), ECSError> {
        self.system_group.add(func)
    }

    pub fn run(&mut self, resources: &Resources) -> Result<(), ECSError> {
        for system in self.system_group.iter() {
            let mut system = system.lock().unwrap();
            system.run(resources)?;
        }
        Ok(())
    }
}
