//! Contains types related to defining shared resources which can be accessed inside systems.
//!
//! Use resources to share persistent data between systems or to provide a system with state
//! external to entities.

use crate::{core::arena::Arena, core::ids::SmallStringId, ECSError};
use downcast_rs::{impl_downcast, Downcast};
use std::{
    any::{self, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
    sync::atomic::{self, AtomicIsize, AtomicUsize},
};

const DEFAULT_PAGE_SIZE: usize = 16;
static STORE_UNIQUE_ID: AtomicUsize = AtomicUsize::new(0);

pub type ResourceTag = SmallStringId<16>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemId(usize);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceId {
    Global,
    Tag(ResourceTag),
    System(SystemId),
}

/// Blanket trait for resource types.
pub trait Resource: 'static {}

impl<T> Resource for T where T: 'static {}

/// Shared reference to a resource
pub struct ResourceRead<'store, 'cell, T>
where
    'store: 'cell,
    T: Resource,
{
    /// Keep a readlock on the store, to avoid any "structural" change in the map
    _store: ResourceStoreRead<'store, T>,
    cell: &'cell ResourceCell<T>,
}

impl<'cell, 'store: 'cell, T: Resource> Deref for ResourceRead<'store, 'cell, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cell.ptr }
    }
}

impl<'cell, 'store: 'cell, T: 'cell + Resource + fmt::Debug> fmt::Debug for ResourceRead<'store, 'cell, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl<'cell, 'store: 'cell, T: Resource> Drop for ResourceRead<'store, 'cell, T> {
    fn drop(&mut self) {
        self.cell.read_unlock();
    }
}

/// Unique reference to a resource
pub struct ResourceWrite<'store, 'cell, T>
where
    'store: 'cell,
    T: Resource,
{
    /// Keep a readlock on the store, to avoid any "structural" change in the map
    _store: ResourceStoreRead<'store, T>,
    cell: &'cell ResourceCell<T>,
}

impl<'cell, 'store: 'cell, T: 'cell + Resource> Deref for ResourceWrite<'store, 'cell, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cell.ptr }
    }
}

impl<'cell, 'store: 'cell, T: 'cell + Resource> DerefMut for ResourceWrite<'store, 'cell, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.cell.ptr }
    }
}

impl<'cell, 'store: 'cell, T: 'cell + Resource + fmt::Debug> fmt::Debug for ResourceWrite<'store, 'cell, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl<'cell, 'store: 'cell, T: Resource> Drop for ResourceWrite<'store, 'cell, T> {
    fn drop(&mut self) {
        self.cell.write_unlock();
    }
}

/// Shared reference to multiple resources of the same type
pub struct ResourceMultiRead<'store, 'cell, T>
where
    'store: 'cell,
    T: Resource,
{
    /// Keep a readlock on the store, to avoid any "structural" change in the map
    _store: ResourceStoreRead<'store, T>,
    cells: Vec<&'cell ResourceCell<T>>,
}

impl<'cell, 'store: 'cell, T: Resource> ResourceMultiRead<'store, 'cell, T> {
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

impl<'cell, 'store: 'cell, T: Resource> Index<usize> for ResourceMultiRead<'store, 'cell, T> {
    type Output = T;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        unsafe { &*self.cells[idx].ptr }
    }
}

impl<'cell, 'store: 'cell, T: Resource + fmt::Debug> fmt::Debug for ResourceMultiRead<'store, 'cell, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries((0..self.len()).map(|i| &self[i])).finish()
    }
}

impl<'cell, 'store: 'cell, T: Resource> Drop for ResourceMultiRead<'store, 'cell, T> {
    fn drop(&mut self) {
        self.cells.iter().for_each(|cell| cell.read_unlock());
    }
}

/// Unique reference to multiple resources of the same type (with different id)
pub struct ResourceMultiWrite<'store, 'cell, T>
where
    'store: 'cell,
    T: Resource,
{
    /// Keep a readlock on the store, to avoid any "structural" change in the map
    _store: ResourceStoreRead<'store, T>,
    cells: Vec<&'cell ResourceCell<T>>,
}

impl<'cell, 'store: 'cell, T: Resource> ResourceMultiWrite<'store, 'cell, T> {
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}
/*
struct ResourceMultiWriteIterator<'r, 'store, 'cell, T>
where
    'store: 'cell,
    T: Resource
{
    res: &'r ResourceMultiWrite<'store, 'cell, T>,
    id: usize
}

impl<'r, 'cell, 'store: 'cell, T: Resource> IntoIterator for &'r ResourceMultiWrite<'store, 'cell, T> {
    type Item = &'r T;


}
*/
impl<'cell, 'store: 'cell, T: Resource> Index<usize> for ResourceMultiWrite<'store, 'cell, T> {
    type Output = T;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        unsafe { &*self.cells[idx].ptr }
    }
}

impl<'cell, 'store: 'cell, T: Resource> IndexMut<usize> for ResourceMultiWrite<'store, 'cell, T> {
    #[inline]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        unsafe { &mut *self.cells[idx].ptr }
    }
}

impl<'cell, 'store: 'cell, T: Resource + fmt::Debug> fmt::Debug for ResourceMultiWrite<'store, 'cell, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries((0..self.len()).map(|i| &self[i])).finish()
    }
}

impl<'cell, 'store: 'cell, T: Resource> Drop for ResourceMultiWrite<'store, 'cell, T> {
    fn drop(&mut self) {
        self.cells.iter().for_each(|cell| cell.write_unlock());
    }
}

/// Storage of single resource instance
struct ResourceCell<T: Resource> {
    arena_alloc: usize,
    ptr: *mut T,
    borrow_state: AtomicIsize,
}

impl<T: Resource> ResourceCell<T> {
    fn read_lock(&self) {
        loop {
            let read = self.borrow_state.load(atomic::Ordering::SeqCst);
            if read < 0 {
                panic!("Resource of {} already borrowed as mutable", any::type_name::<T>());
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

    fn read_unlock(&self) {
        self.borrow_state.fetch_sub(1, atomic::Ordering::Relaxed);
    }

    fn write_lock(&self) {
        let borrowed = self.borrow_state.compare_and_swap(0, -1, atomic::Ordering::SeqCst);
        match borrowed {
            0 => {}
            x if x < 0 => panic!("Resource of {} already borrowed as mutable", any::type_name::<T>()),
            _ => panic!("Resource of {} already borrowed as immutable", any::type_name::<T>()),
        }
    }

    fn write_unlock(&self) {
        self.borrow_state.fetch_add(1, atomic::Ordering::Relaxed);
    }
}

/// Store resources of the same type (with different id)
pub struct ResourceStore<T: Resource> {
    generation: usize,
    arena: Arena<T>,
    map: HashMap<ResourceId, ResourceCell<T>>,
}

impl<T: Resource> ResourceStore<T> {
    fn new() -> Self {
        Self {
            generation: STORE_UNIQUE_ID.fetch_add(1, atomic::Ordering::Relaxed), //todo: atomic inc
            arena: Arena::new(DEFAULT_PAGE_SIZE),
            map: Default::default(),
        }
    }

    pub fn contains(&self, id: &ResourceId) -> bool {
        self.map.contains_key(id)
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    unsafe fn insert(&mut self, id: ResourceId, resource: T) {
        let (arena_alloc, resource) = self.arena.allocate(resource);
        self.map.insert(
            id,
            ResourceCell {
                arena_alloc,
                ptr: resource as *mut _,
                borrow_state: AtomicIsize::new(0),
            },
        );
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    unsafe fn remove(&mut self, id: &ResourceId) -> Option<T> {
        let cell = self.map.remove(id);
        cell.map(|cell| {
            assert!(cell.borrow_state.load(atomic::Ordering::Relaxed) == 0);
            self.arena.deallocate(cell.arena_alloc)
        })
    }
}

/// Helper trait to help implementing downcast for RespurceStore
trait StoreCast: Downcast {}
impl<T: Resource> StoreCast for ResourceStore<T> {}
impl_downcast!(StoreCast);

/// Shared access to the container of the resources of a single type.
/// This accessor ensures, no new resources are created and thus
/// ResourceCells are safe to use/access.
pub struct ResourceStoreRead<'store, T: Resource> {
    state: &'store AtomicIsize,
    inner: &'store ResourceStore<T>,
}

impl<'store, T: Resource> Clone for ResourceStoreRead<'store, T> {
    fn clone(&self) -> Self {
        // already locked for read, no need to worry about different type of concurent locking
        self.state.fetch_add(1, atomic::Ordering::Relaxed);
        Self {
            state: self.state,
            inner: self.inner,
        }
    }
}

impl<'store, T: Resource> ResourceStoreRead<'store, T> {
    pub fn get_with_id<'cell>(&self, id: &ResourceId) -> Result<ResourceRead<'store, 'cell, T>, ECSError> {
        let cell = self
            .inner
            .map
            .get(id)
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?;
        cell.read_lock();
        Ok(ResourceRead {
            _store: self.clone(),
            cell,
        })
    }

    pub fn get<'cell>(&self) -> Result<ResourceRead<'store, 'cell, T>, ECSError> {
        self.get_with_id(&ResourceId::Global)
    }

    pub fn get_with_ids<'cell, 'i, I: IntoIterator<Item = &'i ResourceId>>(
        &self,
        ids: I,
    ) -> Result<ResourceMultiRead<'store, 'cell, T>, ECSError> {
        let cells = ids
            .into_iter()
            .map(|id| {
                self.inner
                    .map
                    .get(id)
                    .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        cells.iter().for_each(|cell| cell.read_lock());
        Ok(ResourceMultiRead {
            _store: self.clone(),
            cells,
        })
    }

    pub fn get_mut_with_id<'cell>(&self, id: &ResourceId) -> Result<ResourceWrite<'store, 'cell, T>, ECSError> {
        let cell = self
            .inner
            .map
            .get(id)
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?;
        cell.write_lock();
        Ok(ResourceWrite {
            _store: self.clone(),
            cell,
        })
    }

    pub fn get_mut<'cell>(&self) -> Result<ResourceWrite<'store, 'cell, T>, ECSError> {
        self.get_mut_with_id(&ResourceId::Global)
    }

    pub fn get_mut_with_ids<'cell, 'i, I: IntoIterator<Item = &'i ResourceId>>(
        &self,
        ids: I,
    ) -> Result<ResourceMultiWrite<'store, 'cell, T>, ECSError> {
        let cells = ids
            .into_iter()
            .map(|id| {
                self.inner
                    .map
                    .get(id)
                    .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        cells.iter().for_each(|cell| cell.write_lock());
        Ok(ResourceMultiWrite {
            _store: self.clone(),
            cells,
        })
    }
}

impl<'store, T: Resource> Deref for ResourceStoreRead<'store, T> {
    type Target = ResourceStore<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'store, T: Resource> Drop for ResourceStoreRead<'store, T> {
    fn drop(&mut self) {
        self.state.fetch_sub(1, atomic::Ordering::Relaxed);
    }
}

/// Unique reference to the container of the resources of a single type.
pub struct ResourceStoreWrite<'store, T: Resource> {
    state: &'store AtomicIsize,
    inner: &'store mut ResourceStore<T>,
}

impl<'store, T: 'store + Resource> Deref for ResourceStoreWrite<'store, T> {
    type Target = ResourceStore<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<'store, T: 'store + Resource> DerefMut for ResourceStoreWrite<'store, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut ResourceStore<T> {
        &mut *self.inner
    }
}

impl<'store, T: Resource> Drop for ResourceStoreWrite<'store, T> {
    fn drop(&mut self) {
        self.state.fetch_add(1, atomic::Ordering::Relaxed);
    }
}

/// Storage of a ResourceStore
struct ResourceStoreCell {
    resources: UnsafeCell<Box<dyn StoreCast>>,
    borrow_state: AtomicIsize,
}

impl ResourceStoreCell {
    fn new<T: Resource>() -> Self {
        Self {
            resources: UnsafeCell::new(Box::new(ResourceStore::<T>::new())),
            borrow_state: AtomicIsize::new(0),
        }
    }

    /// # Safety
    /// Types which are !Sync should only be created on the thread which owns the resource
    /// collection.
    unsafe fn insert<T: Resource>(&mut self, id: ResourceId, resource: T) {
        self.resources
            .get()
            .as_mut()
            .expect("Unsafe error")
            .downcast_mut::<ResourceStore<T>>()
            .expect("Downcast error")
            .insert(id, resource);
    }

    /// # Safety
    /// Types which are !Sync should only be retrieved on the thread which owns the resource
    /// collection.
    unsafe fn remove<T: Resource>(&mut self, id: &ResourceId) -> Option<T> {
        self.resources
            .get()
            .as_mut()
            .expect("Unsafe error")
            .downcast_mut::<ResourceStore<T>>()
            .expect("Downcast error")
            .remove(id)
    }

    /// # Safety
    /// Types which are !Sync should only be retrieved on the thread which owns the resource
    /// collection.
    unsafe fn read_store<T: Resource>(&self) -> ResourceStoreRead<'_, T> {
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

        let resource = self
            .resources
            .get()
            .as_ref()
            .expect("Unsafe error")
            .downcast_ref::<ResourceStore<T>>()
            .expect("Downcast error");

        ResourceStoreRead {
            state: &self.borrow_state,
            inner: resource,
        }
    }

    /// # Safety
    /// Types which are !Send should only be retrieved on the thread which owns the resource
    /// collection.
    unsafe fn write_store<T: Resource>(&self) -> ResourceStoreWrite<'_, T> {
        let borrowed = self.borrow_state.compare_and_swap(0, -1, atomic::Ordering::SeqCst);
        match borrowed {
            0 => {
                let resource = self
                    .resources
                    .get()
                    .as_mut()
                    .expect("Unsafe error")
                    .downcast_mut::<ResourceStore<T>>()
                    .expect("Downcast error");

                ResourceStoreWrite {
                    state: &self.borrow_state,
                    inner: resource,
                }
            }
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
}

/// Store all the resources. Unsafe as the Send and Sync property of a resource is not
/// respected.
#[derive(Default)]
struct UnsafeResources {
    map: HashMap<TypeId, ResourceStoreCell>,
}

unsafe impl Send for UnsafeResources {}
unsafe impl Sync for UnsafeResources {}

impl UnsafeResources {
    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    unsafe fn insert<T: Resource>(&mut self, id: ResourceId, resource: T) {
        let ty = TypeId::of::<T>();
        self.map
            .entry(ty)
            .or_insert_with(ResourceStoreCell::new::<T>)
            .insert::<T>(id, resource);
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    unsafe fn remove<T: Resource>(&mut self, id: &ResourceId) -> Option<T> {
        let ty = TypeId::of::<T>();
        self.map.get_mut(&ty)?.remove::<T>(id)
    }

    unsafe fn read_store<T: Resource>(&self) -> Option<ResourceStoreRead<'_, T>> {
        let ty = TypeId::of::<T>();
        Some(self.map.get(&ty)?.read_store::<T>())
    }

    unsafe fn write_store<T: Resource>(&self) -> Option<ResourceStoreWrite<'_, T>> {
        let ty = TypeId::of::<T>();
        Some(self.map.get(&ty)?.write_store::<T>())
    }
}

/// Resources container.
#[derive(Default)]
pub struct Resources {
    internal: UnsafeResources,
    // marker to make `Resources` !Send and !Sync
    _not_send_sync: PhantomData<*const u8>,
}

impl Resources {
    /*/// Creates an accessor to resources which are Send and Sync and can be sent
    /// safely between threads.
    pub fn sync(&mut self) -> SyncResources {
        SyncResources {
            internal: &self.internal,
        }
    }*/

    /// Inserts an instance of `T` into the store with the given id
    pub fn insert_with_id<T: Resource>(&mut self, id: ResourceId, value: T) {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe {
            self.internal.insert(id, value);
        }
    }

    /// Inserts the instance of `T` into the store.
    pub fn insert<T: Resource>(&mut self, value: T) {
        self.insert_with_id(ResourceId::Global, value);
    }

    /// Removes the instance of `T` with the given id from this store if it exists.    
    pub fn remove_with_id<T: Resource>(&mut self, id: &ResourceId) -> Option<T> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.remove::<T>(id) }
    }

    /// Removes the type `T` from this store if it exists.    
    pub fn remove<T: Resource>(&mut self) -> Option<T> {
        self.remove_with_id::<T>(&ResourceId::Global)
    }

    pub fn get_store<T: Resource>(&self) -> Option<ResourceStoreRead<'_, T>> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.read_store::<T>() }
    }

    pub fn get_store_mut<T: Resource>(&self) -> Option<ResourceStoreWrite<'_, T>> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.write_store::<T>() }
    }

    pub fn get_with_id<'cell, 'store: 'cell, 'r: 'store, T: Resource>(
        &'r self,
        id: &ResourceId,
    ) -> Result<ResourceRead<'store, 'cell, T>, ECSError> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.read_store::<T>() }
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?
            .get_with_id(id)
    }

    pub fn get<'cell, 'store: 'cell, 'r: 'store, T: Resource>(
        &'r self,
    ) -> Result<ResourceRead<'store, 'cell, T>, ECSError> {
        self.get_with_id::<T>(&ResourceId::Global)
    }

    pub fn get_mut_with_id<'cell, 'store: 'cell, 'r: 'store, T: Resource>(
        &'r self,
        id: &ResourceId,
    ) -> Result<ResourceWrite<'store, 'cell, T>, ECSError> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.read_store::<T>() }
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?
            .get_mut_with_id(id)
    }

    pub fn get_mut<'cell, 'store: 'cell, 'r: 'store, T: Resource>(
        &'r self,
    ) -> Result<ResourceWrite<'store, 'cell, T>, ECSError> {
        self.get_mut_with_id::<T>(&ResourceId::Global)
    }
}
