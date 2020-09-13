//! Contains types related to defining shared resources which can be accessed inside systems.
//!
//! Use resources to share persistent data between systems or to provide a system with state
//! external to entities.

use crate::core::ids::SmallStringId;
use downcast_rs::{impl_downcast, Downcast};
use std::{
    any::{self, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    fmt,
    hash::Hasher,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
    sync::atomic::{self, AtomicIsize},
};

pub type ResourceName = SmallStringId<16>;

/// Unique ID for a resource.
#[derive(Clone, Debug, Eq, PartialOrd, Ord)]
pub struct ResourceIndex {
    type_id: TypeId,
    name: Option<ResourceName>,

    #[cfg(debug_assertions)]
    type_name: &'static str,
}

impl ResourceIndex {
    /// Returns the resource type ID of the given resource type.
    pub fn of<T: Resource>(name: Option<ResourceName>) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            name,

            #[cfg(debug_assertions)]
            type_name: any::type_name::<T>(),
        }
    }
}

impl std::hash::Hash for ResourceIndex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.name.hash(state);
    }
}

impl PartialEq for ResourceIndex {
    fn eq(&self, other: &Self) -> bool {
        self.type_id.eq(&other.type_id) && self.name == other.name
    }
}

impl fmt::Display for ResourceIndex {
    #[cfg(debug_assertions)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{:?}]", self.type_name, self.name)
    }

    #[cfg(not(debug_assertions))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}[{:?}]", self.type_id, self.name)
    }
}

/// Blanket trait for resource types.
pub trait Resource: 'static + Downcast {}

impl<T> Resource for T where T: 'static {}
impl_downcast!(Resource);

/// Fetches a shared resource reference
pub struct ResourceRead<'a, T: Resource> {
    state: &'a AtomicIsize,
    inner: &'a T,
}

impl<'a, T: Resource> Deref for ResourceRead<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, T: 'a + Resource + fmt::Debug> fmt::Debug for ResourceRead<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl<'a, T: Resource> Drop for ResourceRead<'a, T> {
    fn drop(&mut self) {
        self.state.fetch_sub(1, atomic::Ordering::Relaxed);
    }
}

/// Fetches a unique resource reference
pub struct ResourceWrite<'a, T: Resource> {
    state: &'a AtomicIsize,
    inner: &'a mut T,
}

impl<'a, T: 'a + Resource> Deref for ResourceWrite<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<'a, T: 'a + Resource> DerefMut for ResourceWrite<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.inner
    }
}

impl<'a, T: 'a + Resource + fmt::Debug> fmt::Debug for ResourceWrite<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl<'a, T: Resource> Drop for ResourceWrite<'a, T> {
    fn drop(&mut self) {
        self.state.fetch_add(1, atomic::Ordering::Relaxed);
    }
}

/// Fetches multiple (distinct) shared resource references of the same resource type
pub struct MultiResourceRead<'a, T: Resource> {
    inner: Vec<ResourceRead<'a, T>>,
}

impl<'a, T: Resource> MultiResourceRead<'a, T> {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<'a, T: Resource> Index<usize> for MultiResourceRead<'a, T> {
    type Output = T;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.inner[idx]
    }
}

impl<'a, T: 'a + Resource + fmt::Debug> fmt::Debug for MultiResourceRead<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.inner).finish()
    }
}

/// Fetches multiple (distinct) unique resource references of the same resource type
pub struct MultiResourceWrite<'a, T: Resource> {
    inner: Vec<ResourceWrite<'a, T>>,
}

impl<'a, T: Resource> MultiResourceWrite<'a, T> {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<'a, T: Resource> Index<usize> for MultiResourceWrite<'a, T> {
    type Output = T;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.inner[idx]
    }
}

impl<'a, T: Resource> IndexMut<usize> for MultiResourceWrite<'a, T> {
    #[inline]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.inner[idx]
    }
}

impl<'a, T: 'a + Resource + fmt::Debug> fmt::Debug for MultiResourceWrite<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.inner).finish()
    }
}

/// A resource with its borrow state simmilar to RefCell but for resources.
pub struct ResourceCell {
    data: UnsafeCell<Box<dyn Resource>>,
    //access_id: AccessId,
    borrow_state: AtomicIsize,
}

impl ResourceCell {
    fn new(resource: Box<dyn Resource>) -> Self {
        Self {
            data: UnsafeCell::new(resource),
            borrow_state: AtomicIsize::new(0),
        }
    }

    fn into_inner(self) -> Box<dyn Resource> {
        self.data.into_inner()
    }

    /// # Safety
    /// Types which are !Sync should only be retrieved on the thread which owns the resource
    /// collection.
    pub unsafe fn get<T: Resource>(&self) -> Option<ResourceRead<'_, T>> {
        loop {
            let read = self.borrow_state.load(atomic::Ordering::SeqCst);
            if read < 0 {
                panic!("resource already borrowed as mutable: {}", any::type_name::<T>());
            }

            if self
                .borrow_state
                .compare_and_swap(read, read + 1, atomic::Ordering::SeqCst)
                == read
            {
                break;
            }
        }

        let resource = self.data.get().as_ref().and_then(|r| r.downcast_ref::<T>());
        if let Some(resource) = resource {
            Some(ResourceRead {
                state: &self.borrow_state,
                inner: resource,
            })
        } else {
            self.borrow_state.fetch_sub(1, atomic::Ordering::Relaxed);
            None
        }
    }

    /// # Safety
    /// Types which are !Send should only be retrieved on the thread which owns the resource
    /// collection.
    pub unsafe fn get_mut<T: Resource>(&self) -> Option<ResourceWrite<'_, T>> {
        let borrowed = self.borrow_state.compare_and_swap(0, -1, atomic::Ordering::SeqCst);
        match borrowed {
            0 => {
                let resource = self.data.get().as_mut().and_then(|r| r.downcast_mut::<T>());
                if let Some(resource) = resource {
                    Some(ResourceWrite {
                        state: &self.borrow_state,
                        inner: resource,
                    })
                } else {
                    self.borrow_state.fetch_add(1, atomic::Ordering::Relaxed);
                    None
                }
            }
            x if x < 0 => panic!("resource already borrowed as mutable: {}", any::type_name::<T>()),
            _ => panic!("resource already borrowed as immutable: {}", any::type_name::<T>()),
        }
    }
}

/// A container for resources which performs runtime borrow checking
/// but _does not_ ensure that `!Sync` resources aren't accessed across threads.
#[derive(Default)]
pub struct UnsafeResources {
    map: HashMap<ResourceIndex, ResourceCell>,
}

unsafe impl Send for UnsafeResources {}
unsafe impl Sync for UnsafeResources {}

impl UnsafeResources {
    fn contains(&self, type_id: &ResourceIndex) -> bool {
        self.map.contains_key(type_id)
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    unsafe fn insert<T: Resource>(&mut self, name: Option<ResourceName>, resource: T) {
        self.map
            .insert(ResourceIndex::of::<T>(name), ResourceCell::new(Box::new(resource)));
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    unsafe fn remove(&mut self, type_id: &ResourceIndex) -> Option<Box<dyn Resource>> {
        self.map.remove(type_id).map(|cell| cell.into_inner())
    }

    fn get(&self, type_id: &ResourceIndex) -> Option<&ResourceCell> {
        self.map.get(type_id)
    }
}

/// Resources container. Shared resources stored here can be retrieved in systems.
#[derive(Default)]
pub struct Resources {
    internal: UnsafeResources,
    // marker to make `Resources` !Send and !Sync
    _not_send_sync: PhantomData<*const u8>,
}

impl Resources {
    /// Creates an accessor to resources which are Send and Sync, which itself can be sent
    /// between threads.
    pub fn sync(&mut self) -> SyncResources {
        SyncResources {
            internal: &self.internal,
        }
    }

    /// Returns `true` if type `T` exists in the store. Otherwise, returns `false`.
    pub fn contains<T: Resource>(&self, name: Option<ResourceName>) -> bool {
        self.internal
            .contains(&ResourceIndex::of::<T>(name.map(|n| n.to_owned())))
    }

    /// Inserts the instance of `T` into the store. If the type already exists, it will be silently
    /// overwritten. If you would like to retain the instance of the resource that already exists,
    /// call `remove` first to retrieve it.
    pub fn insert<T: Resource>(&mut self, name: Option<ResourceName>, value: T) {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe {
            self.internal.insert(name, value);
        }
    }

    /// Removes the type `T` from this store if it exists.
    ///
    /// # Returns
    /// If the type `T` was stored, the inner instance of `T is returned. Otherwise, `None`.
    pub fn remove<T: Resource>(&mut self, name: &Option<ResourceName>) -> Option<T> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe {
            let resource = self
                .internal
                .remove(&ResourceIndex::of::<T>(name.as_ref().map(|n| n.to_owned())))?
                .downcast::<T>()
                .ok()?;
            Some(*resource)
        }
    }

    /// Retrieve an immutable reference to  `T` from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get<T: Resource>(&self, name: &Option<ResourceName>) -> Option<ResourceRead<'_, T>> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        let type_id = &ResourceIndex::of::<T>(name.as_ref().map(|n| n.to_owned()));
        unsafe { self.internal.get(&type_id)?.get::<T>() }
    }

    /// Retrieve a mutable reference to  `T` from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed.
    pub fn get_mut<T: Resource>(&self, name: &Option<ResourceName>) -> Option<ResourceWrite<'_, T>> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        let type_id = &ResourceIndex::of::<T>(name.as_ref().map(|n| n.to_owned()));
        unsafe { self.internal.get(&type_id)?.get_mut::<T>() }
    }

    /// Retrieve a list of immutable reference to  `T` from the store if it all of them exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get_multi<T: Resource>(&self, names: &[Option<ResourceName>]) -> Option<MultiResourceRead<'_, T>> {
        log::debug!("get_multi: {:?}", names);
        let mut resources = Vec::with_capacity(names.len());
        for name in names {
            match self.get::<T>(name) {
                Some(res) => resources.push(res),
                None => return None,
            }
        }
        Some(MultiResourceRead { inner: resources })
    }

    /// Retrieve a list of mutable reference to  `T` from the store if all of them exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed.
    pub fn get_multi_mut<T: Resource>(&self, names: &[Option<ResourceName>]) -> Option<MultiResourceWrite<'_, T>> {
        log::debug!("get_multi_mut: {:?}", names);
        let mut resources = Vec::with_capacity(names.len());
        for name in names {
            match self.get_mut::<T>(name) {
                Some(res) => resources.push(res),
                None => return None,
            }
        }
        Some(MultiResourceWrite { inner: resources })
    }
}

/// A resource collection which is `Send` and `Sync`, but which only allows access to resources
/// which are `Sync`.
pub struct SyncResources<'a> {
    internal: &'a UnsafeResources,
}

impl<'a> SyncResources<'a> {
    /// Retrieve an immutable reference to  `T` from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get<T: Resource + Sync>(&self, name: &Option<ResourceName>) -> Option<ResourceRead<'_, T>> {
        // safety:
        // only resources which are Sync can be accessed, and so are safe to access from any thread
        let type_id = &ResourceIndex::of::<T>(name.as_ref().map(|n| n.to_owned()));
        unsafe { self.internal.get(&type_id)?.get::<T>() }
    }

    /// Retrieve a mutable reference to  `T` from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed.
    pub fn get_mut<T: Resource + Send>(&self, name: &Option<ResourceName>) -> Option<ResourceWrite<'_, T>> {
        // safety:
        // only resources which are Send can be accessed, and so are safe to access from any thread
        let type_id = &ResourceIndex::of::<T>(name.as_ref().map(|n| n.to_owned()));
        unsafe { self.internal.get(&type_id)?.get_mut::<T>() }
    }

    /// Retrieve a list of immutable reference to  `T` from the store if it all of them exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get_multi<T: Resource + Sync>(&self, names: &[Option<ResourceName>]) -> Option<MultiResourceRead<'_, T>> {
        let mut resources = Vec::with_capacity(names.len());
        for name in names {
            match self.get::<T>(name) {
                Some(res) => resources.push(res),
                None => return None,
            }
        }
        Some(MultiResourceRead { inner: resources })
    }

    /// Retrieve a list of mutable reference to  `T` from the store if all of them exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed.
    pub fn get_multi_mut<T: Resource + Send>(
        &self,
        names: &[Option<ResourceName>],
    ) -> Option<MultiResourceWrite<'_, T>> {
        let mut resources = Vec::with_capacity(names.len());
        for name in names {
            match self.get_mut::<T>(name) {
                Some(res) => resources.push(res),
                None => return None,
            }
        }
        Some(MultiResourceWrite { inner: resources })
    }
}
