use shine_ecs::ecs::{
    resources::{Resource, ResourceHandle, ResourceRead, ResourceTag, ResourceWrite, Resources},
    scheduler::Schedule,
    ECSError,
};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorldError {
    #[error("Plugin {0} is missing a resource: {1}")]
    MissingPluginResource(String, ResourceHandle),

    #[error(transparent)]
    ECSError(ECSError),
}

#[derive(Default)]
pub struct World {
    pub resources: Resources,
    schedules: HashMap<String, Schedule>,
}

impl World {
    /// Helper to get a shared reference to a resources
    pub fn plugin_resource<T: Resource>(&self, plugin: &str) -> Result<ResourceRead<'_, T>, WorldError> {
        match self.resources.get::<T>() {
            Ok(res) => Ok(res),
            Err(ECSError::ResourceNotFound(id)) => WorldError::MissingPluginResource(plugin.into(), id),
            Err(err) => WorldError::ECSError(err),
        }
    }

    /// Helper to get a shared reference to a resource with the given tag
    pub fn plugin_resource_with_tag<T: Resource>(
        &self,
        plugin: &str,
        tag: &ResourceTag,
    ) -> Result<ResourceRead<'_, T>, WorldError> {
        match self.resources.get_with_tag::<T>(tag) {
            Ok(res) => Ok(res),
            Err(ECSError::ResourceNotFound(id)) => WorldError::MissingPluginResource(plugin.into(), id),
            Err(err) => WorldError::ECSError(err),
        }
    }

    /// Helper to get an unique reference to a resource
    pub fn plugin_resource_mut<T: Resource>(&self, plugin: &str) -> Result<ResourceWrite<'_, T>, WorldError> {
        match self.resources.get_mut::<T>() {
            Ok(res) => Ok(res),
            Err(ECSError::ResourceNotFound(id)) => WorldError::MissingPluginResource(plugin.into(), id),
            Err(err) => WorldError::ECSError(err),
        }
    }

    /// Helper to get an unique reference to a resource with the given tag
    pub fn plugin_resource_mut_with_tag<T: Resource>(
        &self,
        plugin: &str,
        tag: &ResourceTag,
    ) -> Result<ResourceWrite<'_, T>, WorldError> {
        match self.resources.get_mut_with_tag::<T>(tag) {
            Ok(res) => Ok(res),
            Err(ECSError::ResourceNotFound(id)) => WorldError::MissingPluginResource(plugin.into(), id),
            Err(err) => WorldError::ECSError(err),
        }
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
