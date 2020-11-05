//! Contains types related to defining shared resources which can be accessed inside systems.
//!
//! Use resources to share persistent data between systems or to provide a system with state
//! external to entities.

use crate::{
    resources::{
        Resource, ResourceCell, ResourceHandle, ResourceId, ResourceMultiRead, ResourceMultiWrite, ResourceRead,
        ResourceWrite,
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

/// Atomic cunter to generate unique id for each store, thus ResourceHandle can be bound
/// to a store
static STORE_UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

/// Store resources of the same type (with different id)
pub(crate) struct ResourceStore<T: Resource> {
    generation: usize,
    map: HashMap<ResourceId, Arc<ResourceCell<T>>>,
    pending: Mutex<HashMap<ResourceId, Arc<ResourceCell<T>>>>,

    /// Optional functor to create missing resources from id
    build: Option<Box<dyn Fn(&ResourceId) -> T>>,
    /// Remove unreferenced resources during maintain
    auto_gc: bool,
}

impl<T: Resource> ResourceStore<T> {
    fn new(build: Option<Box<dyn Fn(&ResourceId) -> T>>) -> Self {
        let auto_gc = build.is_some();
        Self {
            generation: STORE_UNIQUE_ID.fetch_add(1, atomic::Ordering::Relaxed),
            map: Default::default(),
            pending: Mutex::new(Default::default()),
            build,
            auto_gc,
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
        self.map.contains_key(id) // stored in the usual map
         || self.build.is_some() // has a builder, thus it will be constructed even is it does not exists at the moment
    }

    /// Insert a new resource. If a resource with the given id already exists,
    /// all the handles are invalidated.
    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the thread owning the resources.
    pub unsafe fn insert(&mut self, id: ResourceId, resource: T) -> Option<T> {
        let out = self.remove(&id);
        self.map.insert(id, ResourceCell::new(resource));
        out
    }

    /// Remove a resource and invalidate all the handles.
    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the thread owning the resources.
    pub unsafe fn remove(&mut self, id: &ResourceId) -> Option<T> {
        let cell = self.pending.lock().unwrap().remove(&id);
        let cell = cell.or_else(|| self.map.remove(&id));

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
    /// Types which are !Sync should only be retrieved or inserted on the thread which owns the resource collection.
    /// Without the build functionality it'd be safe to access the cell on any thread,
    /// but as it is allowed to create new resources, this function should be called only from
    /// thread owning resource.
    pub unsafe fn get_cell(&self, id: &ResourceId) -> Option<Arc<ResourceCell<T>>> {
        self.map.get(id).cloned().or_else(|| {
            self.build.as_ref().map(|build| {
                let mut pending = self.pending.lock().unwrap();
                let cell = pending
                    .entry(id.clone())
                    .or_insert_with_key(|id| ResourceCell::new((build)(id)));
                cell.clone()
            })
        })
    }

    /// # Safety
    /// Types which are !Sync should only be retrieved on the thread which owns the resource collection.
    pub unsafe fn batch_process<'i, I, F>(&mut self, handles: I, process: F)
    where
        I: IntoIterator<Item = &'i ResourceHandle<T>>,
        F: Fn(&mut T),
    {
        for handle in handles {
            if let Some(cell) = handle.upgrade() {
                if handle.generation() == self.generation {
                    debug_assert!(Arc::ptr_eq(self.map.get(&handle.id()).unwrap(), &cell));
                    cell.write_lock();
                    (process)(cell.write());
                    cell.write_unlock();
                }
            }
        }
    }

    /// Move resources from pending into the permanent map.
    /// # Safety
    /// As this operation moves only the
    /// Arc object, but cell remains pinned in memory, it is safe to call for any resources on any thread
    /// dispite of the Send, Sync properties.
    pub fn bake(&mut self) {
        {
            let mut pending = self.pending.lock().unwrap();
            self.map.extend(pending.drain());
        }
        if self.auto_gc {
            self.map.retain(|_, entry| entry.hash_handle());
        }
    }
}

/// Storage of a ResourceStore
pub(crate) struct ResourceStoreCell<T: Resource> {
    store: UnsafeCell<ResourceStore<T>>,
    borrow_state: AtomicIsize,
}

impl<T: Resource> ResourceStoreCell<T> {
    pub fn new(build: Option<Box<dyn Fn(&ResourceId) -> T>>) -> Self {
        Self {
            store: UnsafeCell::new(ResourceStore::new(build)),
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
        let p = self.borrow_state.fetch_sub(1, atomic::Ordering::Relaxed);
        debug_assert!(p > 0);
    }

    #[inline]
    pub fn read(&self) -> &ResourceStore<T> {
        debug_assert!(self.borrow_state.load(atomic::Ordering::Relaxed) > 0);
        // safety:
        //  borrow_state ensures the appropriate lock
        //  resources are not touched here, Sedn, Sync is a different level of safety requirement
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
        let p = self.borrow_state.fetch_add(1, atomic::Ordering::Relaxed);
        debug_assert!(p == -1);
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn write(&self) -> &mut ResourceStore<T> {
        debug_assert!(self.borrow_state.load(atomic::Ordering::Relaxed) < 0);
        // safety:
        //  borrow_state ensures the appropriate lock
        //  resources are not touched here, Sedn, Sync is a different level of safety requirement
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
        //  this type can be constructed only if the required Send and Sync properties are fullfiled
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
            Err(ECSError::ResourceHandleAlien)
        } else if let Some(cell) = handle.upgrade() {
            Ok(ResourceRead::new(self.clone(), cell))
        } else {
            Err(ECSError::ResourceHandleNotFound(any::type_name::<T>().into()))
        }
    }

    pub fn at_mut(&self, handle: &ResourceHandle<T>) -> Result<ResourceWrite<'store, T>, ECSError> {
        if handle.generation() != self.generation() {
            Err(ECSError::ResourceHandleAlien)
        } else if let Some(cell) = handle.upgrade() {
            Ok(ResourceWrite::new(self.clone(), cell))
        } else {
            Err(ECSError::ResourceHandleNotFound(any::type_name::<T>().into()))
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
        //  this type can be constructed only if the required Send and Sync properties are fullfiled
        unsafe { self.store_mut().insert(id, resource) }
    }

    pub fn remove(&mut self, id: &ResourceId) -> Option<T> {
        // safety:
        //  this type can be constructed only if the required Send and Sync properties are fullfiled
        unsafe { self.store_mut().remove(id) }
    }

    pub fn bake(&mut self) {
        self.store_mut().bake();
    }

    pub fn bake_with_process<'i, I, F>(&mut self, handles: I, process: F)
    where
        I: IntoIterator<Item = &'i ResourceHandle<T>>,
        F: Fn(&mut T),
    {
        let store = self.store_mut();
        store.bake();
        
        // safety:
        //  this type can be constructed only if the required Send and Sync properties are fullfiled
        unsafe {
            store.batch_process(handles, process);
        }
    }
}
