//! Contains types related to defining shared resources which can be accessed inside systems.
//!
//! Use resources to share persistent data between systems or to provide a system with state
//! external to entities.

use crate::resources::{Resource, ResourceCell, ResourceId};
use std::{
    any, fmt,
    sync::{Arc, Weak},
};

/// Direct index to a resource. The resource can be access through
/// the resource store accessors [ResourceStoreRead] and [ResourceStoreWrite],
/// but instead of a hash based lookup, direct pointer access is used.
/// For more info see [Resource::insert_managed].
pub struct ResourceHandle<T: Resource> {
    generation: usize,
    cell: Weak<ResourceCell<T>>,

    #[cfg(debug_assertions)]
    id: ResourceId,
}

impl<T: Resource> Clone for ResourceHandle<T> {
    fn clone(&self) -> Self {
        if let Some(cell) = self.upgrade() {
            cell.add_handle();
        }
        Self {
            generation: self.generation,
            cell: self.cell.clone(),
            #[cfg(debug_assertions)]
            id: self.id.clone(),
        }
    }
}

impl<T: Resource> Drop for ResourceHandle<T> {
    fn drop(&mut self) {
        if let Some(cell) = self.upgrade() {
            cell.remove_handle();
        }
    }
}

impl<T: Resource> ResourceHandle<T> {
    pub(crate) fn new(generation: usize, cell: &Arc<ResourceCell<T>>, id: &ResourceId) -> Self {
        cell.add_handle();
        Self {
            generation,
            cell: Arc::downgrade(cell),
            #[cfg(debug_assertions)]
            id: id.clone(),
        }
    }

    pub fn generation(&self) -> usize {
        self.generation
    }

    pub fn is_alive(&self) -> bool {
        self.cell.strong_count() > 0
    }

    #[cfg(debug_assertions)]
    pub fn id(&self) -> &ResourceId {
        &self.id
    }

    pub(crate) fn upgrade(&self) -> Option<Arc<ResourceCell<T>>> {
        self.cell.upgrade()
    }
}

impl<T: Resource> fmt::Debug for ResourceHandle<T> {
    #[cfg(debug_assertions)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ResourceHandle")
            .field(&self.id)
            .field(&any::type_name::<T>().to_owned())
            .finish()
    }

    #[cfg(not(debug_assertions))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ResourceHandle").field(any::type_name::<T>()).finish()
    }
}
