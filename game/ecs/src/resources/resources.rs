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
    convert::TryInto,
    fmt,
    hash::Hasher,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
    sync::atomic::{self, AtomicIsize},
};

pub type ResourceTag = SmallStringId<16>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemId(usize);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceId {
    Global,
    Tag(ResourceTag),
    System(SystemId),
}

/// Unique ID for a resource.
#[derive(Clone, Debug, Eq, PartialOrd, Ord)]
pub struct ResourceHandle {
    type_id: TypeId,
    id: ResourceId,

    #[cfg(debug_assertions)]
    type_name: &'static str,
}

impl ResourceHandle {
    /// Returns the resource type ID of the given resource type.
    pub fn new<T: Resource>(id: ResourceId) -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            id,

            #[cfg(debug_assertions)]
            type_name: any::type_name::<T>(),
        }
    }

    pub fn id(&self) -> &ResourceId {
        &self.id
    }
}

impl std::hash::Hash for ResourceHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.type_id.hash(state);
        self.id.hash(state);
    }
}

impl PartialEq for ResourceHandle {
    fn eq(&self, other: &Self) -> bool {
        self.type_id.eq(&other.type_id) && self.id == other.id
    }
}

impl fmt::Display for ResourceHandle {
    #[cfg(debug_assertions)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{:?}]", self.type_name, self.id)
    }

    #[cfg(not(debug_assertions))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}[{:?}]", self.type_id, self.id)
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
pub struct TaggedResourceRead<'a, T: Resource> {
    inner: Vec<ResourceRead<'a, T>>,
}

impl<'a, T: Resource> TaggedResourceRead<'a, T> {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<'a, T: Resource> Index<usize> for TaggedResourceRead<'a, T> {
    type Output = T;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.inner[idx]
    }
}

impl<'a, T: 'a + Resource + fmt::Debug> fmt::Debug for TaggedResourceRead<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.inner).finish()
    }
}

/// Fetches multiple (distinct) unique resource references of the same resource type
pub struct TaggedResourceWrite<'a, T: Resource> {
    inner: Vec<ResourceWrite<'a, T>>,
}

impl<'a, T: Resource> TaggedResourceWrite<'a, T> {
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl<'a, T: Resource> Index<usize> for TaggedResourceWrite<'a, T> {
    type Output = T;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        &self.inner[idx]
    }
}

impl<'a, T: Resource> IndexMut<usize> for TaggedResourceWrite<'a, T> {
    #[inline]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.inner[idx]
    }
}

impl<'a, T: 'a + Resource + fmt::Debug> fmt::Debug for TaggedResourceWrite<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.inner).finish()
    }
}

/// A resource with its borrow state simmilar to RefCell but for resources.
pub struct ResourceCell {
    data: UnsafeCell<Box<dyn Resource>>,
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
    map: HashMap<ResourceHandle, ResourceCell>,
}

unsafe impl Send for UnsafeResources {}
unsafe impl Sync for UnsafeResources {}

impl UnsafeResources {
    fn contains(&self, type_id: &ResourceHandle) -> bool {
        self.map.contains_key(type_id)
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    unsafe fn insert<T: Resource>(&mut self, id: ResourceId, resource: T) {
        self.map
            .insert(ResourceHandle::new::<T>(id), ResourceCell::new(Box::new(resource)));
    }

    /// # Safety
    /// Resources which are `!Send` must be retrieved or inserted only on the main thread.
    unsafe fn remove(&mut self, type_id: &ResourceHandle) -> Option<Box<dyn Resource>> {
        self.map.remove(type_id).map(|cell| cell.into_inner())
    }

    fn get(&self, type_id: &ResourceHandle) -> Option<&ResourceCell> {
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
    /// Creates an accessor to resources which are Send and Sync and can be sent
    /// safely between threads.
    pub fn sync(&mut self) -> SyncResources {
        SyncResources {
            internal: &self.internal,
        }
    }

    fn insert_impl<T: Resource>(&mut self, id: ResourceId, value: T) {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe {
            self.internal.insert(id, value);
        }
    }

    /// Inserts the instance of `T` into the store.
    pub fn insert<T: Resource>(&mut self, value: T) {
        self.insert_impl(ResourceId::Global, value);
    }

    /// Inserts the instance of `T` with the given tag into the store.
    pub fn insert_with_tag<T: Resource>(&mut self, tag: ResourceTag, value: T) {
        self.insert_impl(ResourceId::Tag(tag), value);
    }

    /// Inserts the instance of `T` with the given tag into the store.
    pub fn insert_with_try_tag<G: TryInto<ResourceTag>, T: Resource>(
        &mut self,
        tag: G,
        value: T,
    ) -> Result<(), <G as TryInto<ResourceTag>>::Error> {
        self.insert_impl(ResourceId::Tag(tag.try_into()?), value);
        Ok(())
    }

    /*/// Inserts the instance of `T` for each system. As resource is created on
    /// demand when requested, T have to implement Default.
    pub fn insert_local<T: Default + Resource>(&mut self) {
        unimplemented!()
    }*/

    fn contains_impl<T: Resource>(&self, id: ResourceId) -> bool {
        self.internal.contains(&ResourceHandle::new::<T>(id))
    }

    /// Returns if type `T` exists in the store.
    pub fn contains<T: Resource>(&self) -> bool {
        self.contains_impl::<T>(ResourceId::Global)
    }

    /// Returns if type `T` with the given tag exists in the store.
    pub fn contains_with_tag<T: Resource>(&self, tag: &ResourceTag) -> bool {
        self.contains_impl::<T>(ResourceId::Tag(tag.to_owned()))
    }

    fn remove_impl<T: Resource>(&mut self, id: ResourceId) -> Option<T> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        unsafe {
            let resource = self
                .internal
                .remove(&ResourceHandle::new::<T>(id))?
                .downcast::<T>()
                .ok()?;
            Some(*resource)
        }
    }

    /// Removes the type `T` from this store if it exists.    
    pub fn remove<T: Resource>(&mut self) -> Option<T> {
        self.remove_impl::<T>(ResourceId::Global)
    }

    /// Removes the type `T` with the given tag from this store if it exists.    
    pub fn remove_with_tag<T: Resource>(&mut self, tag: &ResourceTag) -> Option<T> {
        self.remove_impl::<T>(ResourceId::Tag(tag.to_owned()))
    }

    fn get_impl<T: Resource>(&self, id: ResourceId) -> Option<ResourceRead<'_, T>> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        let type_id = &ResourceHandle::new::<T>(id);
        unsafe { self.internal.get(&type_id)?.get::<T>() }
    }

    /// Retrieve an shared reference to `T` from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get<T: Resource>(&self) -> Option<ResourceRead<'_, T>> {
        self.get_impl::<T>(ResourceId::Global)
    }

    /// Retrieve an shared reference to a `T` with the give tag from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get_with_tag<T: Resource>(&self, tag: &ResourceTag) -> Option<ResourceRead<'_, T>> {
        self.get_impl::<T>(ResourceId::Tag(tag.to_owned()))
    }

    /// Retrieve a list of shared reference to `T` with the given tags from the store
    /// if it all of them exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get_with_tags<'a, T: Resource, I: IntoIterator<Item = &'a ResourceTag>>(
        &'a self,
        tags: I,
    ) -> Option<TaggedResourceRead<'_, T>> {
        let inner = tags
            .into_iter()
            .map(|tag| self.get_with_tag::<T>(tag))
            .collect::<Option<Vec<_>>>()?;
        Some(TaggedResourceRead { inner })
    }

    fn get_mut_impl<T: Resource>(&self, id: ResourceId) -> Option<ResourceWrite<'_, T>> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        let type_id = &ResourceHandle::new::<T>(id);
        unsafe { self.internal.get(&type_id)?.get_mut::<T>() }
    }

    /// Retrieve a unique reference to `T` from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed.
    pub fn get_mut<T: Resource>(&self) -> Option<ResourceWrite<'_, T>> {
        self.get_mut_impl::<T>(ResourceId::Global)
    }

    /// Retrieve a unique reference to a `T` with the given tag from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed.
    pub fn get_mut_with_tag<T: Resource>(&self, tag: &ResourceTag) -> Option<ResourceWrite<'_, T>> {
        self.get_mut_impl::<T>(ResourceId::Tag(tag.to_owned()))
    }

    /// Retrieve a list of unique references to `T` with the given tags from the store
    /// if it all of them exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get_mut_with_tags<'a, T: Resource, I: IntoIterator<Item = &'a ResourceTag>>(
        &'a self,
        tags: I,
    ) -> Option<TaggedResourceWrite<'_, T>> {
        let inner = tags
            .into_iter()
            .map(|tag| self.get_mut_with_tag::<T>(tag))
            .collect::<Option<Vec<_>>>()?;
        Some(TaggedResourceWrite { inner })
    }
}

/// A resource collection which is `Send` and `Sync`, but which only allows access to resources
/// which are `Sync`.
pub struct SyncResources<'a> {
    internal: &'a UnsafeResources,
}

impl<'a> SyncResources<'a> {
    fn get_impl<T: Resource + Sync>(&self, id: ResourceId) -> Option<ResourceRead<'_, T>> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        let type_id = &ResourceHandle::new::<T>(id);
        unsafe { self.internal.get(&type_id)?.get::<T>() }
    }

    /// Retrieve an shared reference to `T` from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get<T: Resource + Sync>(&self) -> Option<ResourceRead<'_, T>> {
        self.get_impl::<T>(ResourceId::Global)
    }

    /// Retrieve an shared reference to a `T` with the give tag from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get_with_tag<T: Resource + Sync>(&self, tag: &ResourceTag) -> Option<ResourceRead<'_, T>> {
        self.get_impl::<T>(ResourceId::Tag(tag.to_owned()))
    }

    /// Retrieve a list of shared reference to `T` with the given tags from the store
    /// if it all of them exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get_with_tags<'t, T: Resource + Sync, I: IntoIterator<Item = &'t ResourceTag>>(
        &'t self,
        tags: I,
    ) -> Option<TaggedResourceRead<'_, T>> {
        let inner = tags
            .into_iter()
            .map(|tag| self.get_with_tag::<T>(tag))
            .collect::<Option<Vec<_>>>()?;
        Some(TaggedResourceRead { inner })
    }

    fn get_mut_impl<T: Resource + Send>(&self, id: ResourceId) -> Option<ResourceWrite<'_, T>> {
        // safety:
        // this type is !Send and !Sync, and so can only be accessed from the thread which
        // owns the resources collection
        let type_id = &ResourceHandle::new::<T>(id);
        unsafe { self.internal.get(&type_id)?.get_mut::<T>() }
    }

    /// Retrieve a unique reference to `T` from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed.
    pub fn get_mut<T: Resource + Send>(&self) -> Option<ResourceWrite<'_, T>> {
        self.get_mut_impl::<T>(ResourceId::Global)
    }

    /// Retrieve a unique reference to a `T` with the given tag from the store if it exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed.
    pub fn get_mut_with_tag<T: Resource + Send>(&self, tag: &ResourceTag) -> Option<ResourceWrite<'_, T>> {
        self.get_mut_impl::<T>(ResourceId::Tag(tag.to_owned()))
    }

    /// Retrieve a list of unique references to `T` with the given tags from the store
    /// if it all of them exists.
    /// Otherwise, return `None`.
    ///
    /// # Panics
    /// Panics if the resource is already borrowed mutably.
    pub fn get_mut_with_tags<'t, T: Resource + Send, I: IntoIterator<Item = &'t ResourceTag>>(
        &'t self,
        tags: I,
    ) -> Option<TaggedResourceWrite<'_, T>> {
        let inner = tags
            .into_iter()
            .map(|tag| self.get_mut_with_tag::<T>(tag))
            .collect::<Option<Vec<_>>>()?;
        Some(TaggedResourceWrite { inner })
    }
}
