//! Contains types related to defining shared resources which can be accessed inside systems.
//!
//! Use resources to share persistent data between systems or to provide a system with state
//! external to entities.

use crate::{
    resources::{Resource, ResourceCell, ResourceId, ResourceStoreRead, ResourceStoreWrite},
    ECSError,
};
use std::{
    any, fmt,
    ops::{Deref, DerefMut, Index, IndexMut},
};

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

impl<'cell, 'store: 'cell, T: Resource> ResourceRead<'store, 'cell, T> {
    pub(crate) fn new(store: ResourceStoreRead<'store, T>, cell: &'cell ResourceCell<T>) -> Self {
        ResourceRead { _store: store, cell }
    }
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

impl<'cell, 'store: 'cell, T: Resource> ResourceWrite<'store, 'cell, T> {
    pub(crate) fn new(store: ResourceStoreRead<'store, T>, cell: &'cell ResourceCell<T>) -> Self {
        ResourceWrite { _store: store, cell }
    }
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

/// Implement resource read/write based access operations
impl<'store, T: Resource> ResourceStoreRead<'store, T> {
    pub fn contains(&self, id: &ResourceId) -> bool {
        self.get_cell(id).is_some()
    }

    pub fn get<'cell>(&self) -> Result<ResourceRead<'store, 'cell, T>, ECSError> {
        self.get_with_id(&ResourceId::Global)
    }

    pub fn get_with_id<'cell>(&self, id: &ResourceId) -> Result<ResourceRead<'store, 'cell, T>, ECSError> {
        let cell = self
            .get_cell(id)
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?;
        cell.read_lock();
        Ok(ResourceRead {
            _store: self.clone(),
            cell,
        })
    }

    pub fn get_with_ids<'cell, 'i, I: IntoIterator<Item = &'i ResourceId>>(
        &self,
        ids: I,
    ) -> Result<ResourceMultiRead<'store, 'cell, T>, ECSError> {
        let store = self.clone();
        let cells = ids
            .into_iter()
            .map(|id| {
                store
                    .get_cell(id)
                    .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        cells.iter().for_each(|cell| cell.read_lock());
        Ok(ResourceMultiRead { _store: store, cells })
    }

    pub fn get_mut<'cell>(&self) -> Result<ResourceWrite<'store, 'cell, T>, ECSError> {
        self.get_mut_with_id(&ResourceId::Global)
    }

    pub fn get_mut_with_id<'cell>(&self, id: &ResourceId) -> Result<ResourceWrite<'store, 'cell, T>, ECSError> {
        let store = self.clone();
        let cell = store
            .get_cell(id)
            .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))?;
        cell.write_lock();
        Ok(ResourceWrite { _store: store, cell })
    }

    pub fn get_mut_with_ids<'cell, 'i, I: IntoIterator<Item = &'i ResourceId>>(
        &self,
        ids: I,
    ) -> Result<ResourceMultiWrite<'store, 'cell, T>, ECSError> {
        let store = self.clone();
        let cells = ids
            .into_iter()
            .map(|id| {
                store
                    .get_cell(id)
                    .ok_or_else(|| ECSError::ResourceNotFound(any::type_name::<T>().into(), id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        cells.iter().for_each(|cell| cell.write_lock());
        Ok(ResourceMultiWrite { _store: store, cells })
    }
}

/// Implement resource read/write based access operations
impl<'store, T: Resource> ResourceStoreWrite<'store, T> {
    pub fn contains(&self, id: &ResourceId) -> bool {
        self.get_cell(id).is_some()
    }
}
