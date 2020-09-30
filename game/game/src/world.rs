use shine_ecs::{
    resources::{Resource, ResourceName, ResourceRead, ResourceWrite, Resources},
    scheduler::Schedule,
};
use std::{any, collections::HashMap};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorldError {
    #[error("Plugin {0} was not registered, missing {1}")]
    MissingPlugin(String, String),
}

#[derive(Default)]
pub struct World {
    pub resources: Resources,
    schedules: HashMap<String, Schedule>,
}

impl World {
    /// Helper to get a shared reference to a resources
    pub fn plugin_resource<T: Resource>(&self, plugin: &str) -> Result<ResourceRead<'_, T>, WorldError> {
        self.resources
            .get::<T>()
            .ok_or_else(|| WorldError::MissingPlugin(plugin.into(), any::type_name::<T>().into()))
    }

    /// Helper to get a shared reference to a resource with the given name
    pub fn plugin_resource_with_name<T: Resource>(
        &self,
        plugin: &str,
        name: &ResourceName,
    ) -> Result<ResourceRead<'_, T>, WorldError> {
        self.resources
            .get_with_name::<T>(name)
            .ok_or_else(|| WorldError::MissingPlugin(plugin.into(), any::type_name::<T>().into()))
    }

    /// Helper to get an unique reference to a resource
    pub fn plugin_resource_mut<T: Resource>(&self, plugin: &str) -> Result<ResourceWrite<'_, T>, WorldError> {
        self.resources
            .get_mut::<T>()
            .ok_or_else(|| WorldError::MissingPlugin(plugin.into(), any::type_name::<T>().into()))
    }

    /// Helper to get an unique reference to a resource with the given name
    pub fn plugin_resource_mut_with_name<T: Resource>(
        &self,
        plugin: &str,
        name: &ResourceName,
    ) -> Result<ResourceWrite<'_, T>, WorldError> {
        self.resources
            .get_mut_with_name::<T>(name)
            .ok_or_else(|| WorldError::MissingPlugin(plugin.into(), any::type_name::<T>().into()))
    }

    pub fn add_stage(&mut self, stage: &str, schedule: Schedule) {
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
    }
}
