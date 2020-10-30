//! Contains types related to defining shared resources which can be accessed inside systems.
//!
//! Use resources to share persistent data between systems or to provide a system with state
//! external to entities.

use crate::{
    resources::{Resource, ResourceCell, ResourceId, ResourceRead, ResourceStoreRead, ResourceWrite},
    ECSError,
};
use std::{
    any,
    marker::PhantomData,
    sync::{
        atomic::{self, AtomicUsize},
        Arc, Weak,
    },
};

/// Direct index to a resource. The resource can be access through
/// the resource store accessors [ResourceStoreRead] and [ResourceStoreWrite],
/// but instead of a hash based lookup a direct pointer access is used.
/// For more info see [Resource::insert_managed].
pub struct ResourceHandle<T: Resource> {
    generation: usize,
    cell: *const ResourceCell<T>,
    ref_counter: Weak<AtomicUsize>,
}

impl<T: Resource> ResourceHandle<T> {
    pub fn is_alive(&self) -> bool {
        self.ref_counter.upgrade().is_some()
    }
}

impl<T: Resource> Clone for ResourceHandle<T> {
    fn clone(&self) -> Self {
        // check if resource is still alive and was not dropped
        if let Some(ref_counter) = self.ref_counter.upgrade() {
            ref_counter.fetch_add(1, atomic::Ordering::Relaxed);
        }
        Self {
            generation: self.generation,
            cell: self.cell,
            ref_counter: self.ref_counter.clone(),
        }
    }
}

impl<T: Resource> Drop for ResourceHandle<T> {
    fn drop(&mut self) {
        // check if resource is still alive and was not dropped
        if let Some(ref_counter) = self.ref_counter.upgrade() {
            ref_counter.fetch_sub(1, atomic::Ordering::Relaxed);
        }
    }
}

impl<'store, T: Resource> ResourceStoreRead<'store, T> {
    pub fn get_handle(&self, id: &ResourceId) -> Result<ResourceHandle<T>, ECSError> {
        let cell = self
            .get_cell(id)
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?;
        Ok(ResourceHandle {
            generation: self.generation(),
            ref_counter: Arc::downgrade(cell.ref_counter.as_ref().unwrap()),
            cell,
        })
    }

    pub fn at<'cell>(&self, handle: &ResourceHandle<T>) -> Result<ResourceRead<'store, 'cell, T>, ECSError> {
        if handle.generation == self.generation() && handle.is_alive() {
            let cell = unsafe { &*handle.cell };
            cell.read_lock();
            Ok(ResourceRead::new(self.clone(), cell))
        } else {
            Err(ECSError::ResourceHandleNotFound(any::type_name::<T>().into()))
        }
    }

    pub fn at_mut<'cell>(&self, handle: &ResourceHandle<T>) -> Result<ResourceWrite<'store, 'cell, T>, ECSError> {
        if handle.generation == self.generation() && handle.is_alive() {
            let cell = unsafe { &*handle.cell };
            cell.write_lock();
            Ok(ResourceWrite::new(self.clone(), cell))
        } else {
            Err(ECSError::ResourceHandleNotFound(any::type_name::<T>().into()))
        }
    }
}
