use crate::{core::rwtoken::RWToken, resources::ResourceStoreRead};
use std::{
    any::type_name,
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut, Index, IndexMut},
    sync::{
        atomic::{self, AtomicUsize},
        Arc,
    },
};

/// Blanket trait for resource types.
pub trait Resource: 'static + Sized {}
impl<T> Resource for T where T: 'static {}

/// Storage of single resource instance
pub(crate) struct ResourceCell<T: Resource> {
    resource: UnsafeCell<Option<T>>,
    rw_token: RWToken,
    handle_count: AtomicUsize,
}

unsafe impl<T: Resource> Send for ResourceCell<T> {}
unsafe impl<T: Resource> Sync for ResourceCell<T> {}

impl<T: Resource> ResourceCell<T> {
    pub fn new_occupied(resource: T) -> Arc<Self> {
        Arc::new(ResourceCell {
            resource: UnsafeCell::new(Some(resource)),
            handle_count: AtomicUsize::new(0),
            rw_token: RWToken::new(),
        })
    }

    /// Creates an empty, write locked resource cell .
    pub fn new_empty() -> Arc<Self> {
        Arc::new(ResourceCell {
            resource: UnsafeCell::new(None),
            handle_count: AtomicUsize::new(0),
            rw_token: RWToken::new_write_locked(),
        })
    }

    /// Removes the resource form a cell leaving it empty (and write locked)
    /// Types which are !Send should only be retrieved only on the thread which owns the resource collection.
    pub unsafe fn take(&self) -> T {
        self.write_lock();
        // safety
        //  rw_token ensures the appropriate lock
        let res = &mut *self.resource.get();
        res.take().unwrap()
    }

    /// Set the resource of an empty cell and release the write lock
    pub unsafe fn set(&self, resource: T) {
        debug_assert!(self.rw_token.is_write());
        // safety
        //  rw_token ensures the appropriate lock
        let res = &mut *self.resource.get();
        debug_assert!(res.is_none());
        *res = Some(resource);
        self.rw_token.write_unlock();
    }

    pub fn read_lock(&self) {
        self.rw_token
            .try_read_lock()
            .unwrap_or_else(|err| panic!("Immutable borrow of a resource [{}] failed: {}", type_name::<T>(), err));
    }

    pub fn read_unlock(&self) {
        self.rw_token.read_unlock();
    }

    /// # Safety
    /// Types which are !Sync should only be accessed on the thread which owns the resource collection.
    #[inline]
    pub unsafe fn read(&self) -> &T {
        debug_assert!(self.rw_token.is_read());
        // safety:
        //  rw_token ensures the appropriate lock
        (&*self.resource.get()).as_ref().unwrap()
    }

    pub fn write_lock(&self) {
        self.rw_token
            .try_write_lock()
            .unwrap_or_else(|err| panic!("Mutable borrow of a resource [{}] failed: {}", type_name::<T>(), err))
    }

    pub fn write_unlock(&self) {
        self.rw_token.write_unlock();
    }

    /// # Safety
    /// Types which are !Sync should only be accessed on the thread which owns the resource collection.
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn write(&self) -> &mut T {
        debug_assert!(self.rw_token.is_write());
        // safety:
        // rw_state ensures the appropriate lock
        (&mut *self.resource.get()).as_mut().unwrap()
    }

    pub fn has_handle(&self) -> bool {
        self.handle_count.load(atomic::Ordering::Relaxed) > 0
    }

    pub fn add_handle(&self) {
        self.handle_count.fetch_add(1, atomic::Ordering::Relaxed);
    }

    pub fn remove_handle(&self) {
        self.handle_count.fetch_sub(1, atomic::Ordering::Relaxed);
    }
}

/// Shared reference to a resource
pub struct ResourceRead<'store, T: Resource> {
    /// Keep a readlock on the store, to avoid any "structural" change in the map
    _store: ResourceStoreRead<'store, T>,
    cell: Arc<ResourceCell<T>>,
}

impl<'store, T: Resource> ResourceRead<'store, T> {
    pub(crate) fn new(store: ResourceStoreRead<'store, T>, cell: Arc<ResourceCell<T>>) -> Self {
        cell.read_lock();
        ResourceRead { _store: store, cell }
    }
}

impl<'store, T: Resource> Deref for ResourceRead<'store, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe { self.cell.read() }
    }
}

impl<'store, T: Resource + fmt::Debug> fmt::Debug for ResourceRead<'store, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl<'store, T: Resource> Drop for ResourceRead<'store, T> {
    fn drop(&mut self) {
        self.cell.read_unlock();
    }
}

/// Unique reference to a resource
pub struct ResourceWrite<'store, T: Resource> {
    /// Keep a readlock on the store, to avoid any "structural" change in the map
    _store: ResourceStoreRead<'store, T>,
    cell: Arc<ResourceCell<T>>,
}

impl<'store, T: Resource> ResourceWrite<'store, T> {
    pub(crate) fn new(store: ResourceStoreRead<'store, T>, cell: Arc<ResourceCell<T>>) -> Self {
        cell.write_lock();
        ResourceWrite { _store: store, cell }
    }
}

impl<'store, T: Resource> Deref for ResourceWrite<'store, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe { self.cell.write() }
    }
}

impl<'store, T: Resource> DerefMut for ResourceWrite<'store, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe { self.cell.write() }
    }
}

impl<'store, T: Resource + fmt::Debug> fmt::Debug for ResourceWrite<'store, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl<'store, T: Resource> Drop for ResourceWrite<'store, T> {
    fn drop(&mut self) {
        self.cell.write_unlock();
    }
}

/// Shared reference to multiple resources of the same type
pub struct ResourceMultiRead<'store, T: Resource> {
    /// Keep a readlock on the store, to avoid any "structural" change in the map
    _store: ResourceStoreRead<'store, T>,
    cells: Vec<Arc<ResourceCell<T>>>,
}

impl<'store, T: Resource> ResourceMultiRead<'store, T> {
    pub(crate) fn new(store: ResourceStoreRead<'store, T>, cells: Vec<Arc<ResourceCell<T>>>) -> Self {
        cells.iter().for_each(|cell| cell.read_lock());
        ResourceMultiRead { _store: store, cells }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

impl<'store, T: Resource> Index<usize> for ResourceMultiRead<'store, T> {
    type Output = T;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe { self.cells[idx].read() }
    }
}

impl<'store, T: Resource + fmt::Debug> fmt::Debug for ResourceMultiRead<'store, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries((0..self.len()).map(|i| &self[i])).finish()
    }
}

impl<'store, T: Resource> Drop for ResourceMultiRead<'store, T> {
    fn drop(&mut self) {
        self.cells.iter().for_each(|cell| cell.read_unlock());
    }
}

/// Unique reference to multiple resources of the same type (with different id)
pub struct ResourceMultiWrite<'store, T: Resource> {
    /// Keep a readlock on the store, to avoid any "structural" change in the map
    _store: ResourceStoreRead<'store, T>,
    cells: Vec<Arc<ResourceCell<T>>>,
}

impl<'store, T: Resource> ResourceMultiWrite<'store, T> {
    pub(crate) fn new(store: ResourceStoreRead<'store, T>, cells: Vec<Arc<ResourceCell<T>>>) -> Self {
        cells.iter().for_each(|cell| cell.write_lock());
        ResourceMultiWrite { _store: store, cells }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

impl<'store, T: Resource> Index<usize> for ResourceMultiWrite<'store, T> {
    type Output = T;

    #[inline]
    fn index(&self, idx: usize) -> &Self::Output {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe { self.cells[idx].write() }
    }
}

impl<'store, T: Resource> IndexMut<usize> for ResourceMultiWrite<'store, T> {
    #[inline]
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        // safety:
        //  this type is constructed only if the T implements the required Send and Sync markers
        unsafe { self.cells[idx].write() }
    }
}

impl<'store, T: Resource + fmt::Debug> fmt::Debug for ResourceMultiWrite<'store, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries((0..self.len()).map(|i| &self[i])).finish()
    }
}

impl<'store, T: Resource> Drop for ResourceMultiWrite<'store, T> {
    fn drop(&mut self) {
        self.cells.iter().for_each(|cell| cell.write_unlock());
    }
}
