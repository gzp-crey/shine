use crate::{
    resources::{
        Resource, ResourceCell, ResourceConfig, ResourceHandle, ResourceId, ResourceMultiRead, ResourceMultiWrite,
        ResourceRead, ResourceWrite,
    },
    ECSError,
};
use std::{
    any,
    cell::UnsafeCell,
    collections::HashMap,
    sync::{
        atomic::{self, AtomicIsize, AtomicUsize},
        Arc, Mutex,
    },
};

/// Atomic cuonter to generate unique id for each store, thus ResourceHandle can be bound
/// to a store
static STORE_UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

/// Context for the post process functor called after bake
pub struct ResourceBakeContext<'store, T: Resource> {
    generation: usize,
    resource_map: &'store mut HashMap<ResourceId, Arc<ResourceCell<T>>>,
}

impl<'store, T: Resource> ResourceBakeContext<'store, T> {
    pub fn process_by_handle<F: Fn(&mut T)>(&self, handle: &ResourceHandle<T>, process: F) {
        if let Some(cell) = handle.upgrade() {
            if handle.generation() == self.generation {
                debug_assert!(Arc::ptr_eq(self.resource_map.get(&handle.id()).unwrap(), &cell));
                cell.write_lock();
                // safety:
                //  this type is constructed only if the T implements the required Send and Sync markers
                (process)(unsafe { cell.write() });
                cell.write_unlock();
            }
        }
    }
}

/// Store resources of the same type (with different id)
pub(crate) struct ResourceStore<T: Resource> {
    generation: usize,
    config: <T as Resource>::Config,
    resource_map: HashMap<ResourceId, Arc<ResourceCell<T>>>,
    pending: Mutex<HashMap<ResourceId, Arc<ResourceCell<T>>>>,
}

impl<T: Resource> ResourceStore<T> {
    fn new(config: <T as Resource>::Config) -> Self {
        Self {
            generation: STORE_UNIQUE_ID.fetch_add(1, atomic::Ordering::SeqCst),
            resource_map: Default::default(),
            pending: Mutex::new(Default::default()),
            config,
        }
    }

    pub fn generation(&self) -> usize {
        self.generation
    }

    /// Check if requesting the given resource would be successfull. As some resources are created on demand
    /// they are treated as if they were contained in the store.
    /// # Safety
    /// As this operation does not touch the resources itself, it is safe to call for any resources on any thread
    /// dispite of the Send, Sync properties.
    pub fn contains(&self, id: &ResourceId) -> bool {
        self.resource_map.contains_key(id)   // stored in the usual map
         || self.config.auto_build() // has a builder, thus it will be constructed even is it does not exists at the moment
    }

    /// Insert a new resource. If a resource with the given id already exists, all the handles
    /// are invalidated. The other type of references and accessors are not effected as they
    /// should not exist by the design of the API.
    /// # Safety
    /// Resources which are `!Send` must be inserted only on the thread owning the resources.
    pub unsafe fn insert(&mut self, id: ResourceId, resource: T) -> Option<T> {
        let out = self.remove(&id);
        self.resource_map.insert(id, ResourceCell::new(resource));
        out
    }

    /// Remove a resource and invalidate all the handles. The other type of references and accessors
    /// are not effected as they should not exist by the design of the API.
    /// of the API.
    /// # Safety
    /// Resources which are `!Send` must be retrieved only on the thread owning the resources.
    pub unsafe fn remove(&mut self, id: &ResourceId) -> Option<T> {
        let cell = self.pending.lock().unwrap().remove(&id);
        let cell = cell.or_else(|| self.resource_map.remove(&id));

        if let Some(cell) = cell {
            // No accessor may exits as that would require a &self which contradicts to
            // to rust's borrow checker (have a &self and &mut self at the same time)
            Some(match Arc::try_unwrap(cell) {
                Ok(cell) => cell.take(),
                Err(_) => panic!("Internal error, multiple ref exists to the same resource"),
            })
        } else {
            None
        }
    }

    /// # Safety
    /// Types which are !Send or !Sync should only be accessed on the thread which owns the
    /// resource collection and the resources (and not just the wrapping cells) are
    /// accessed (created) here.
    pub unsafe fn get_cell(&self, id: &ResourceId) -> Option<Arc<ResourceCell<T>>> {
        self.resource_map.get(id).cloned().or_else(|| {
            let config = &self.config;
            if self.config.auto_build() {
                let mut pending = self.pending.lock().unwrap();
                let cell = pending
                    .entry(id.clone())
                    .or_insert_with_key(|id| ResourceCell::new(config.build(id)));
                Some(cell.clone())
            } else {
                None
            }
        })
    }

    /// Move resources from pending into the permanent map.
    /// # Safety
    /// Types which are !Send or !Sync should only be accessed or retrieved on the thread which
    /// owns the resource collection and the resources (and not just the wrapping cells) are
    /// accessed (released or updated).
    pub unsafe fn bake(&mut self) {
        {
            let mut pending = self.pending.lock().unwrap();
            self.resource_map.extend(pending.drain());
        }
        if self.config.auto_gc() {
            self.resource_map.retain(|_, entry| entry.has_handle());
        }
        self.config.post_process(&mut ResourceBakeContext {
            generation: self.generation,
            resource_map: &mut self.resource_map,
        });
    }
}

/// Storage of a ResourceStore
pub(crate) struct ResourceStoreCell<T: Resource> {
    store: UnsafeCell<ResourceStore<T>>,
    borrow_state: AtomicIsize,
}

impl<T: Resource> ResourceStoreCell<T> {
    pub fn new(config: <T as Resource>::Config) -> Self {
        Self {
            store: UnsafeCell::new(ResourceStore::new(config)),
            borrow_state: AtomicIsize::new(0),
        }
    }

    pub fn read_lock(&self) {
        loop {
            let read = self.borrow_state.load(atomic::Ordering::SeqCst);
            if read < 0 {
                panic!(
                    "Resource store for {} already borrowed as mutable",
                    any::type_name::<T>()
                );
            }

            if self
                .borrow_state
                .compare_and_swap(read, read + 1, atomic::Ordering::SeqCst)
                == read
            {
                break;
            }
        }
    }

    pub fn read_unlock(&self) {
        let p = self.borrow_state.fetch_sub(1, atomic::Ordering::SeqCst);
        debug_assert!(p > 0);
    }

    #[inline]
    pub fn read(&self) -> &ResourceStore<T> {
        debug_assert!(self.borrow_state.load(atomic::Ordering::SeqCst) > 0);
        // safety:
        //  borrow_state ensures the appropriate lock
        //  the store itself is Send and Sync (safety of ResourceCell takes care for for T)
        unsafe { &*self.store.get() }
    }

    pub fn write_lock(&self) {
        let borrowed = self.borrow_state.compare_and_swap(0, -1, atomic::Ordering::SeqCst);
        match borrowed {
            0 => {}
            x if x < 0 => panic!(
                "Resource store for {} already borrowed as mutable",
                any::type_name::<T>()
            ),
            _ => panic!(
                "Resource store for {} already borrowed as immutable",
                any::type_name::<T>()
            ),
        }
    }

    pub fn write_unlock(&self) {
        let p = self.borrow_state.fetch_add(1, atomic::Ordering::SeqCst);
        debug_assert!(p == -1);
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn write(&self) -> &mut ResourceStore<T> {
        debug_assert!(self.borrow_state.load(atomic::Ordering::SeqCst) < 0);
        // safety:
        //  borrow_state ensures the appropriate lock
        //  the store itself is Send and Sync (safety of ResourceCell takes care of T)
        unsafe { &mut *self.store.get() }
    }
}

/// Shared access to the resources of a single type.
pub struct ResourceStoreRead<'store, T: Resource> {
    cell: &'store ResourceStoreCell<T>,
}

impl<'store, T: Resource> Clone for ResourceStoreRead<'store, T> {
    fn clone(&self) -> Self {
        self.cell.read_lock();
        Self { cell: self.cell }
    }
}

impl<'store, T: Resource> Drop for ResourceStoreRead<'store, T> {
    fn drop(&mut self) {
        self.cell.read_unlock()
    }
}

impl<'store, T: Resource> ResourceStoreRead<'store, T> {
    pub(crate) fn new(cell: &'store ResourceStoreCell<T>) -> Self {
        cell.read_lock();
        Self { cell }
    }

    fn store(&self) -> &ResourceStore<T> {
        self.cell.read()
    }

    fn get_cell(&self, id: &ResourceId) -> Option<Arc<ResourceCell<T>>> {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe { self.store().get_cell(id) }
    }

    /// Return the unique id of the store
    pub fn generation(&self) -> usize {
        self.store().generation()
    }

    pub fn contains(&self, id: &ResourceId) -> bool {
        self.store().contains(id)
    }

    pub fn get(&self) -> Result<ResourceRead<'store, T>, ECSError> {
        self.get_with_id(&ResourceId::Global)
    }

    pub fn get_with_id(&self, id: &ResourceId) -> Result<ResourceRead<'store, T>, ECSError> {
        let store = self.clone();
        let cell = store
            .get_cell(id)
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?;
        Ok(ResourceRead::new(store, cell))
    }

    pub fn get_with_ids<'i, I: IntoIterator<Item = &'i ResourceId>>(
        &self,
        ids: I,
    ) -> Result<ResourceMultiRead<'store, T>, ECSError> {
        let store = self.clone();
        let cells = ids
            .into_iter()
            .map(|id| {
                store
                    .get_cell(id)
                    .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ResourceMultiRead::new(store, cells))
    }

    pub fn get_mut(&self) -> Result<ResourceWrite<'store, T>, ECSError> {
        self.get_mut_with_id(&ResourceId::Global)
    }

    pub fn get_mut_with_id(&self, id: &ResourceId) -> Result<ResourceWrite<'store, T>, ECSError> {
        let store = self.clone();
        let cell = store
            .get_cell(id)
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?;
        Ok(ResourceWrite::new(store, cell))
    }

    pub fn get_mut_with_ids<'i, I: IntoIterator<Item = &'i ResourceId>>(
        &self,
        ids: I,
    ) -> Result<ResourceMultiWrite<'store, T>, ECSError> {
        let store = self.clone();
        let cells = ids
            .into_iter()
            .map(|id| {
                store
                    .get_cell(id)
                    .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ResourceMultiWrite::new(store, cells))
    }

    pub fn get_handle(&self, id: &ResourceId) -> Result<ResourceHandle<T>, ECSError> {
        let cell = self
            .get_cell(id)
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?;
        Ok(ResourceHandle::new(self.generation(), &cell, id))
    }

    pub fn at(&self, handle: &ResourceHandle<T>) -> Result<ResourceRead<'store, T>, ECSError> {
        if handle.generation() != self.generation() {
            Err(ECSError::ResourceExpired)
        } else if let Some(cell) = handle.upgrade() {
            Ok(ResourceRead::new(self.clone(), cell))
        } else {
            Err(ECSError::ResourceTypeNotFound(any::type_name::<T>().into()))
        }
    }

    pub fn at_mut(&self, handle: &ResourceHandle<T>) -> Result<ResourceWrite<'store, T>, ECSError> {
        if handle.generation() != self.generation() {
            Err(ECSError::ResourceExpired)
        } else if let Some(cell) = handle.upgrade() {
            Ok(ResourceWrite::new(self.clone(), cell))
        } else {
            Err(ECSError::ResourceTypeNotFound(any::type_name::<T>().into()))
        }
    }
}

/// Unique access to the resources of a single type.
pub struct ResourceStoreWrite<'store, T: Resource> {
    cell: &'store ResourceStoreCell<T>,
}

impl<'store, T: Resource> Drop for ResourceStoreWrite<'store, T> {
    fn drop(&mut self) {
        self.cell.write_unlock();
    }
}

impl<'store, T: Resource> ResourceStoreWrite<'store, T> {
    pub(crate) fn new(cell: &'store ResourceStoreCell<T>) -> Self {
        cell.write_lock();
        Self { cell }
    }

    fn store(&self) -> &ResourceStore<T> {
        self.cell.write()
    }

    #[allow(clippy::mut_from_ref)]
    fn store_mut(&self) -> &mut ResourceStore<T> {
        self.cell.write()
    }

    /// Return the unique id of the store
    pub fn generation(&self) -> usize {
        self.store().generation()
    }

    pub fn contains(&self, id: &ResourceId) -> bool {
        self.store().contains(id)
    }

    pub fn insert(&mut self, id: ResourceId, resource: T) -> Option<T> {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe { self.store_mut().insert(id, resource) }
    }

    pub fn remove(&mut self, id: &ResourceId) -> Option<T> {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe { self.store_mut().remove(id) }
    }

    pub fn bake(&mut self) {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe {
            self.store_mut().bake();
        }
    }
}
