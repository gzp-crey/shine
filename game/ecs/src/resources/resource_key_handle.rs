use crate::{
    resources::{Resource, ResourceHandle, ResourceId, ResourceRead, ResourceWrite, Resources},
    ECSError,
};

/// Helper to manage handle to resources with an id of [ResourceId::Binary]
pub struct ResourceKeyHandle<K, T>
where
    K: serde::Serialize,
    T: Resource,
{
    key: K,
    handle: Option<ResourceHandle<T>>,
}

impl<K, T> ResourceKeyHandle<K, T>
where
    K: serde::Serialize,
    T: Resource,
{
    pub fn new(key: K) -> Self {
        Self { key, handle: None }
    }

    pub fn key(&self) -> &K {
        &self.key
    }

    pub fn handle(&self) -> Option<&ResourceHandle<T>> {
        self.handle.as_ref()
    }

    pub fn set(&mut self, key: K) {
        self.key = key;
        self.handle = None;
    }

    fn update_handle(&mut self, resources: &Resources) -> Option<&ResourceHandle<T>> {
        self.handle = ResourceId::from_object(&self.key)
            .and_then(|id| resources.get_handle::<T>(&id))
            .map_err(|err| format!("Failed to get resource: {:?}", err))
            .ok();
        self.handle.as_ref()
    }

    pub fn get<'a>(&mut self, resources: &'a Resources) -> Option<ResourceRead<'a, T>> {
        if let Some(handle) = &self.handle {
            match resources.try_at(handle) {
                Ok(res) => Some(res),
                Err(ECSError::ResourceExpired) => self.update_handle(resources).map(|handle| resources.at(handle)),
                Err(err) => {
                    log::warn!("Failed to get resource handle: {:?}", err);
                    None
                }
            }
        } else {
            self.update_handle(resources).map(|handle| resources.at(handle))
        }
    }

    pub fn get_mut<'a>(&mut self, resources: &'a Resources) -> Option<ResourceWrite<'a, T>> {
        if let Some(handle) = &self.handle {
            match resources.try_at_mut(handle) {
                Ok(res) => Some(res),
                Err(ECSError::ResourceExpired) => self.update_handle(resources).map(|handle| resources.at_mut(handle)),
                Err(err) => {
                    log::warn!("Failed to get resource handle for: {:?}", err);
                    None
                }
            }
        } else {
            self.update_handle(resources).map(|handle| resources.at_mut(handle))
        }
    }
}
