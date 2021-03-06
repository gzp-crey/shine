use crate::{
    core::rwtoken::RWToken,
    dbg_assert,
    resources::{
        Resource, ResourceCell, ResourceConfig, ResourceHandle, ResourceId, ResourceMultiRead, ResourceMultiWrite,
        ResourceRead, ResourceWrite,
    },
    ECSError,
};
use std::{
    any::type_name,
    cell::UnsafeCell,
    collections::HashMap,
    marker::PhantomData,
    sync::{
        atomic::{self, AtomicUsize},
        Arc, Mutex,
    },
};

/// Atomic cuonter to generate unique id for each store, thus ResourceHandle can be bound
/// to a store
static STORE_UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

/// Context for the post process functor called after bake
pub struct ResourceBakeContext<'store, T: Resource> {
    generation: usize,
    _ph: PhantomData<&'store T>,

    #[cfg(debug_assertions)]
    resource_map: &'store mut HashMap<ResourceId, Arc<ResourceCell<T>>>,
}

impl<'store, T: Resource> ResourceBakeContext<'store, T> {
    pub fn process_by_handle<F: FnOnce(&ResourceHandle<T>, &mut T)>(&self, handle: &ResourceHandle<T>, process: F) {
        if let Some(cell) = handle.upgrade() {
            if handle.generation() == self.generation {
                dbg_assert!(Arc::ptr_eq(self.resource_map.get(&handle.id()).unwrap(), &cell));
                cell.write_lock();
                // safety:
                //  this type is constructed only if the T implements the required Send and Sync markers
                (process)(&handle, unsafe { cell.write() });
                cell.write_unlock();
            }
        }
    }
}

/// Store resources of the same type (with different id)
pub(crate) struct ResourceStore<T: Resource> {
    generation: usize,
    config: Box<dyn ResourceConfig<Resource = T>>,
    resource_map: HashMap<ResourceId, Arc<ResourceCell<T>>>,
    pending: Mutex<HashMap<ResourceId, Arc<ResourceCell<T>>>>,
}

impl<T: Resource> ResourceStore<T> {
    fn new(config: Box<dyn ResourceConfig<Resource = T>>) -> Self {
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
        // stored in the usual map
        //  or has a builder, thus it will be constructed even if it does not exists at the moment
        self.resource_map.contains_key(id) || self.config.auto_build()
    }

    /// Check if the the given resource instance exists in the store.
    /// # Safety
    /// As this operation does not touch the resources itself, it is safe to call for any resources on any thread
    /// dispite of the Send, Sync properties.
    pub fn exists(&self, id: &ResourceId) -> bool {
        // stored in the usual map
        //  or stored in the pending set
        self.resource_map.contains_key(id) || self.pending.lock().unwrap().contains_key(id)
    }

    /// Insert a new resource. If a resource with the given id already exists, all the handles
    /// are invalidated. The other type of references and accessors are not effected as they
    /// should not exist by the design of the API.
    /// # Safety
    /// Resources which are `!Send` must be inserted only on the thread owning the resources.
    pub unsafe fn insert(&mut self, id: ResourceId, resource: T) -> Option<T> {
        let out = self.remove(&id);
        self.resource_map.insert(id, ResourceCell::new_occupied(resource));
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

        // No accessor should exits as that would require a &self which contradicts to
        // to rust's borrow checker (have a &self and &mut self at the same time)
        if let Some(cell) = cell {
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
            if self.config.auto_build() {
                let config = &self.config;
                let generation = self.generation();
                let mut pending = self.pending.lock().unwrap();
                let cell = pending.entry(id.clone()).or_insert_with_key(|id| {
                    let cell = ResourceCell::new_empty();
                    let handle = ResourceHandle::new(generation, &cell, &id);
                    cell.set(config.build(handle, id));
                    cell
                });
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
    /// accessed (updated).
    pub unsafe fn bake(&mut self, gc: bool) {
        {
            let mut pending = self.pending.lock().unwrap();
            self.resource_map.extend(pending.drain());
        }
        if gc {
            self.resource_map.retain(|_, entry| entry.has_handle());
        }
        self.config.post_bake(&mut ResourceBakeContext {
            generation: self.generation,
            _ph: PhantomData,
            #[cfg(debug_assertions)]
            resource_map: &mut self.resource_map,
        });
    }
}

/// Storage of a ResourceStore
pub(crate) struct ResourceStoreCell<T: Resource> {
    store: UnsafeCell<ResourceStore<T>>,
    rw_token: RWToken,
}

impl<T: Resource> ResourceStoreCell<T> {
    pub fn new(config: Box<dyn ResourceConfig<Resource = T>>) -> Self {
        Self {
            store: UnsafeCell::new(ResourceStore::new(config)),
            rw_token: RWToken::new(),
        }
    }

    pub fn read_lock(&self) {
        self.rw_token.try_read_lock().unwrap_or_else(|err| {
            panic!(
                "Immutable borrow of a resource store [{}] failed: {}",
                type_name::<T>(),
                err
            )
        });
    }

    pub fn read_unlock(&self) {
        self.rw_token.read_unlock();
    }

    #[inline]
    pub fn read(&self) -> &ResourceStore<T> {
        debug_assert!(self.rw_token.is_read_lock());
        // safety:
        //  rw_token ensures the appropriate lock
        //  the store itself is Send and Sync (safety of ResourceCell takes care for for T)
        unsafe { &*self.store.get() }
    }

    pub fn write_lock(&self) {
        self.rw_token.try_write_lock().unwrap_or_else(|err| {
            panic!(
                "Mutable borrow of a resource store [{}] failed: {}",
                type_name::<T>(),
                err
            )
        });
    }

    pub fn write_unlock(&self) {
        self.rw_token.write_unlock();
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn write(&self) -> &mut ResourceStore<T> {
        debug_assert!(self.rw_token.is_write_lock());
        // safety:
        //  rw_token ensures the appropriate lock
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

    /// Call extension operation on config
    pub fn config(&self) -> &dyn ResourceConfig<Resource = T> {
        &*self.store().config
    }

    pub fn exists(&self, id: &ResourceId) -> bool {
        self.store().exists(id)
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
            .ok_or_else(|| ECSError::ResourceNotFound(type_name::<T>().into(), id.clone()))?;
        Ok(ResourceRead::new(store, cell))
    }

    pub fn get_with_ids<I>(&self, ids: I) -> Result<ResourceMultiRead<'store, T>, ECSError>
    where
        I: IntoIterator,
        I::Item: AsRef<ResourceId>,
    {
        let store = self.clone();
        let cells = ids
            .into_iter()
            .map(|id| {
                store
                    .get_cell(id.as_ref())
                    .ok_or_else(|| ECSError::ResourceNotFound(type_name::<T>().into(), id.as_ref().clone()))
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
            .ok_or_else(|| ECSError::ResourceNotFound(type_name::<T>().into(), id.clone()))?;
        Ok(ResourceWrite::new(store, cell))
    }

    pub fn get_mut_with_ids<I>(&self, ids: I) -> Result<ResourceMultiWrite<'store, T>, ECSError>
    where
        I: IntoIterator,
        I::Item: AsRef<ResourceId>,
    {
        let store = self.clone();
        let cells = ids
            .into_iter()
            .map(|id| {
                store
                    .get_cell(id.as_ref())
                    .ok_or_else(|| ECSError::ResourceNotFound(type_name::<T>().into(), id.as_ref().clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ResourceMultiWrite::new(store, cells))
    }

    pub fn get_handle(&self, id: &ResourceId) -> Result<ResourceHandle<T>, ECSError> {
        let cell = self
            .get_cell(id)
            .ok_or_else(|| ECSError::ResourceNotFound(type_name::<T>().into(), id.clone()))?;
        Ok(ResourceHandle::new(self.generation(), &cell, id))
    }

    pub fn try_at(&self, handle: &ResourceHandle<T>) -> Result<ResourceRead<'store, T>, ECSError> {
        if handle.generation() != self.generation() {
            Err(ECSError::ResourceExpired)
        } else if let Some(cell) = handle.upgrade() {
            Ok(ResourceRead::new(self.clone(), cell))
        } else {
            Err(ECSError::ResourceTypeNotFound(type_name::<T>().into()))
        }
    }

    pub fn at(&self, handle: &ResourceHandle<T>) -> ResourceRead<'store, T> {
        self.try_at(handle).unwrap()
    }

    pub fn try_at_mut(&self, handle: &ResourceHandle<T>) -> Result<ResourceWrite<'store, T>, ECSError> {
        if handle.generation() != self.generation() {
            Err(ECSError::ResourceExpired)
        } else if let Some(cell) = handle.upgrade() {
            Ok(ResourceWrite::new(self.clone(), cell))
        } else {
            Err(ECSError::ResourceTypeNotFound(type_name::<T>().into()))
        }
    }

    pub fn at_mut(&self, handle: &ResourceHandle<T>) -> ResourceWrite<'store, T> {
        self.try_at_mut(handle).unwrap()
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

    #[inline]
    fn store(&self) -> &ResourceStore<T> {
        self.cell.write()
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    fn store_mut(&self) -> &mut ResourceStore<T> {
        self.cell.write()
    }

    /// Return the unique id of the store
    #[inline]
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

    pub fn bake(&mut self, gc: bool) {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe {
            self.store_mut().bake(gc);
        }
    }
}
