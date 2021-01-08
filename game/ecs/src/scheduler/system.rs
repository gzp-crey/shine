use crate::{core::ids::SmallStringId, resources::Resources, scheduler::ResourceClaims, ECSError};
use std::{
    slice::Iter,
    sync::{Arc, Mutex},
};

pub type SystemName = SmallStringId<16>;

/// Systems scheduled for execution
pub trait System: Send + Sync {
    /// name used for trace and debug logs
    fn debug_name(&self) -> &str;

    /// Name of the system to create explicit dependencies
    fn name(&self) -> &Option<SystemName>;
    // Explicit dependency, those must complete before this system
    //fn dependencies(&self) -> &Vec<SystemName>;

    /// Resources claims. Claim shall not change once scheduler execution was started.
    fn resource_claims(&self) -> &ResourceClaims;

    /// Execute the task. On completion it can request a new set of system to be executed.
    fn run(&mut self, resources: &Resources) -> Result<SystemGroup, ECSError>;
}

/// Trait to convert anything into a System.
/// The R genereic parameter is a tuple of all the resource queries
pub trait IntoSystem<R> {
    fn into_system(self) -> Result<Box<dyn System>, ECSError>;
}

/// Trait to convert anything into a (system) Builder. Before constructing the system one may add extra
/// scheduling parameters.
pub trait IntoSystemBuilder<R> {
    type Builder: IntoSystem<R>;

    #[must_use]
    fn system(self) -> Self::Builder;
}

/// A group of systems
#[derive(Default)]
pub struct SystemGroup {
    systems: Vec<Arc<Mutex<Box<dyn System>>>>,
}

impl SystemGroup {
    pub fn add<R, S: IntoSystem<R>>(&mut self, sys: S) -> Result<(), ECSError> {
        self.add_system(sys.into_system()?);
        Ok(())
    }

    pub fn add_system(&mut self, sys: Box<dyn System>) {
        self.systems.push(Arc::new(Mutex::new(sys)));
    }

    pub fn iter(&self) -> Iter<'_, Arc<Mutex<Box<dyn System>>>> {
        self.systems.iter()
    }
}
