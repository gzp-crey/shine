//! Contains types related to defining shared resources which can be accessed inside systems.
//!
//! Use resources to share persistent data between systems or to provide a system with state
//! external to entities.

use crate::{
    core::arena::Arena,
    core::ids::SmallStringId,
    resources::{ResourceMultiRead, ResourceMultiWrite, ResourceRead, ResourceWrite},
    ECSError,
};
use downcast_rs::{impl_downcast, Downcast};
use std::{
    any::{self, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    marker::PhantomData,
    sync::{
        atomic::{self, AtomicIsize, AtomicUsize},
        Arc, Mutex,
    },
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

/// Storage of single resource instance
pub(crate) struct ResourceCell<T: Resource> {
    arena_alloc: usize,
    pub(crate) ptr: *mut T,
    pub(crate) borrow_state: AtomicIsize,
    /// Counts the handlebased references
    pub(crate) ref_counter: Option<Arc<AtomicUsize>>,
}

impl<T: Resource> ResourceCell<T> {
    pub fn read_lock(&self) {
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

    pub fn read_unlock(&self) {
        self.borrow_state.fetch_sub(1, atomic::Ordering::Relaxed);
    }

    pub fn write_lock(&self) {
        let borrowed = self.borrow_state.compare_and_swap(0, -1, atomic::Ordering::SeqCst);
        match borrowed {
            0 => {}
            x if x < 0 => panic!("Resource of {} already borrowed as mutable", any::type_name::<T>()),
            _ => panic!("Resource of {} already borrowed as immutable", any::type_name::<T>()),
        }
    }

    pub fn write_unlock(&self) {
        self.borrow_state.fetch_add(1, atomic::Ordering::Relaxed);
    }
}

/// Store resources of the same type (with different id)
pub struct ResourceStore<T: Resource> {
    generation: usize,
    arena: Arena<T>,
    map: HashMap<ResourceId, ResourceCell<T>>,
    manage: Option<ResourceManage<T>>,
}

impl<T: Resource> ResourceStore<T> {
    fn new(manage: Option<ResourceManage<T>>) -> Self {
        Self {
            generation: STORE_UNIQUE_ID.fetch_add(1, atomic::Ordering::Relaxed), //todo: atomic inc
            arena: Arena::new(DEFAULT_PAGE_SIZE),
            map: Default::default(),
            manage,
        }
    }

    pub fn generation(&self) -> usize {
        self.generation
    }

    pub fn contains(&self, id: &ResourceId) -> bool {
        self.map.contains_key(id)
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    pub(crate) unsafe fn insert(&mut self, id: ResourceId, resource: T) {
        let (arena_alloc, resource) = self.arena.allocate(resource);
        let ref_counter = self.manage.as_ref().map(|_| Arc::new(AtomicUsize::new(0)));
        self.map.insert(
            id,
            ResourceCell {
                arena_alloc,
                ptr: resource as *mut _,
                borrow_state: AtomicIsize::new(0),
                ref_counter,
            },
        );
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    pub(crate) unsafe fn remove(&mut self, id: &ResourceId) -> Option<T> {
        let cell = self.map.remove(id);
        cell.map(|cell| {
            assert!(cell.borrow_state.load(atomic::Ordering::Relaxed) == 0);
            self.arena.deallocate(cell.arena_alloc)
        })
    }

    /// # Safety
    /// - Types which are !Sync should only be retrieved on the thread which owns the resource
    ///   collection.
    /// - The cell is allocated in the arena and "pined" in memory
    ///   the created cell is either dropped when there are no more references (in gc)
    ///   or moved into the map (in bake)
    pub(crate) unsafe fn get_cell(&self, id: &ResourceId) -> Option<&ResourceCell<T>> {
        self.map.get(id).or_else(|| {
            // try to create resource if a builder is provided
            self.manage
                .as_ref()
                .map(|manage| std::mem::transmute(manage.get_or_create(id)))
        })
    }
}

/// Helper trait to help implementing downcast for RespurceStore
trait StoreCast: Downcast {}
impl<T: Resource> StoreCast for ResourceStore<T> {}
impl_downcast!(StoreCast);

struct ResourceManage<T: Resource> {
    /// Resources created while store was read locked.
    pending: Mutex<HashMap<ResourceId, ResourceCell<T>>>,
    build: Box<dyn Fn(&ResourceId) -> T>,
    _ph: PhantomData<T>,
}

impl<T: Resource> ResourceManage<T> {
    fn new<F: 'static + Fn(&ResourceId) -> T>(build: F) -> Self {
        Self {
            pending: Mutex::new(Default::default()),
            build: Box::new(build),
            _ph: PhantomData,
        }
    }

    fn get_or_create(&self, id: &ResourceId) -> &ResourceCell<T> {
        let pending = self.pending.lock().unwrap();
        //pendig.got_or_insert(id)
        unimplemented!()
    }
}

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
    /// Return the unique id of the store
    pub fn generation(&self) -> usize {
        self.inner.generation()
    }

    /// Get a reference to a cell. The returned reference can outlive
    /// &self as it is bound to the lifetime of the store ('store). As a comparision
    /// check [ResourceStoreWrite::get_cell]
    pub(crate) fn get_cell<'cell>(&self, id: &ResourceId) -> Option<&'cell ResourceCell<T>>
    where
        'store: 'cell,
    {
        // saftey:
        //  ResourceStoreRead ensure the Send and Sync properties
        unsafe { self.inner.get_cell(id) }
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

impl<'store, T: Resource> ResourceStoreWrite<'store, T> {
    /// Return the unique id of the store
    pub fn generation(&self) -> usize {
        self.inner.generation()
    }

    /// Get a reference to a cell. The returned reference can outlive &self
    /// as other calls may mutate the store and hence cell could point to
    /// invalid memory location. As a comparision check [ResourceStoreRead::get_cell]
    pub(crate) fn get_cell(&self, id: &ResourceId) -> Option<&ResourceCell<T>> {
        unsafe { self.inner.get_cell(id) }
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
    fn new<T: Resource>(manage: Option<ResourceManage<T>>) -> Self {
        Self {
            resources: UnsafeCell::new(Box::new(ResourceStore::<T>::new(manage))),
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
    unsafe fn create_managed<T: Resource>(&mut self, manage: ResourceManage<T>) {
        let ty = TypeId::of::<T>();
        // Managed store have to be registered using the insert_managed
        // function before instances of the resource can be added
        assert!(self.map.get(&ty).is_none());
        self.map.insert(ty, ResourceStoreCell::new::<T>(Some(manage)));
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    unsafe fn insert<T: Resource>(&mut self, id: ResourceId, resource: T) {
        let ty = TypeId::of::<T>();
        self.map
            .entry(ty)
            .or_insert_with(|| ResourceStoreCell::new::<T>(None))
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

    /// Insert a managed resource, that allow
    /// - ref counter based garbage collection
    /// - creation from id
    /// - TBD: configurable async (background) loading
    pub fn insert_managed<T: Resource, F: 'static + Fn(&ResourceId) -> T>(&mut self, build: F) {
        unsafe {
            self.internal.create_managed::<T>(ResourceManage::new(build));
        }
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

    pub fn get<'cell, 'store: 'cell, 'r: 'store, T: Resource>(
        &'r self,
    ) -> Result<ResourceRead<'store, 'cell, T>, ECSError> {
        self.get_with_id::<T>(&ResourceId::Global)
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

    pub fn get_with_ids<'cell, 'store: 'cell, 'r: 'store, 'i, T: Resource, I: IntoIterator<Item = &'i ResourceId>>(
        &'r self,
        ids: I,
    ) -> Result<ResourceMultiRead<'store, 'cell, T>, ECSError> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.read_store::<T>() }
            .ok_or_else(|| ECSError::ResourceTypeNotFound(any::type_name::<T>().into()))?
            .get_with_ids(ids)
    }

    pub fn get_mut<'cell, 'store: 'cell, 'r: 'store, T: Resource>(
        &'r self,
    ) -> Result<ResourceWrite<'store, 'cell, T>, ECSError> {
        self.get_mut_with_id::<T>(&ResourceId::Global)
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

    pub fn get_mut_with_ids<
        'cell,
        'store: 'cell,
        'r: 'store,
        'i,
        T: Resource,
        I: IntoIterator<Item = &'i ResourceId>,
    >(
        &'r self,
        ids: I,
    ) -> Result<ResourceMultiWrite<'store, 'cell, T>, ECSError> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe { self.internal.read_store::<T>() }
            .ok_or_else(|| ECSError::ResourceTypeNotFound(any::type_name::<T>().into()))?
            .get_mut_with_ids(ids)
    }
}
