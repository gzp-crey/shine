use crate::{
    core::ids::SmallStringId,
    resources::Resources,
    scheduler::{ResourceClaims, TaskGroup},
    ECSError,
};

pub type SystemName = SmallStringId<16>;

/// Systems scheduled for execution
pub trait System: Send + Sync {
    /// name used for trace and debug logs
    fn debug_name(&self) -> &str;

    /// Name of the system to create explicit dependencies
    fn name(&self) -> Option<&SystemName>;
    // Explicit dependency, those must complete before this system
    //fn dependencies(&self) -> &Vec<SystemName>;

    /// Collect and return resources claims.  
    fn resource_claims(&mut self) -> Result<&ResourceClaims, ECSError>;

    /// Execute the task. On completion it can request a new set of system to be executed.
    fn run(&mut self, resources: &Resources) -> Result<TaskGroup, ECSError>;
}

/// Trait to convert anything into a System.
/// The R generic parameter is a tuple of all the resource queries
pub trait IntoSystem<R> {
    type System: System;

    fn into_system(self) -> Self::System;
}
