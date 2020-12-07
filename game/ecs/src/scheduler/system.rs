use crate::{core::ids::SmallStringId, resources::Resources, scheduler::ResourceClaims, ECSError};
use std::{
    slice::Iter,
    sync::{Arc, Mutex},
};

pub type SystemName = SmallStringId<16>;

/// Systems scheduled for execution
pub trait System: Send + Sync {
    fn debug_name(&self) -> &str;
    fn name(&self) -> &Option<SystemName>;
    //fn dependencies(&self) -> &Vec<SystemName>;
    fn resource_claims(&self) -> &ResourceClaims;
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
