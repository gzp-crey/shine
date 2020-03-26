use crate::store::unnamedstore::Store as InnerStore;
use shred::{Read, ResourceId, SystemData, Write};
use std::ops::{Deref, DerefMut};

pub use crate::store::unnamedstore::{Index, ReadGuard, WriteGuard};

/// A wrapper around [InnerStore](InnerStore) to make it more ergonomic for the world use
pub struct Store<D> {
    inner: InnerStore<D>,
}

impl<D> Store<D> {
    pub fn new() -> Store<D> {
        Store {
            inner: InnerStore::new(),
        }
    }

    /// Creates a new store with memory allocated for at least capacity items
    pub fn new_with_capacity(page_size: usize, capacity: usize) -> Store<D> {
        Store {
            inner: InnerStore::new_with_capacity(page_size, capacity),
        }
    }

    pub fn read(&self) -> ReadGuard<'_, D> {
        self.inner.try_read().unwrap()
    }

    pub fn write(&mut self) -> WriteGuard<'_, D> {
        self.inner.try_write().unwrap()
    }
}

impl<D> Default for Store<D> {
    fn default() -> Self {
        Store::new()
    }
}

/// Grant immutable access to a (unnamed) store inside a System
pub struct ReadStore<'a, D>
where
    D: 'static,
{
    inner: Read<'a, Store<D>>,
}

impl<'a, D> Deref for ReadStore<'a, D>
where
    D: 'static,
{
    type Target = Store<D>;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a, D> SystemData<'a> for ReadStore<'a, D>
where
    D: 'static,
{
    fn setup(_: &mut shred::World) {}

    fn fetch(res: &'a shred::World) -> Self {
        ReadStore {
            inner: res.fetch::<Store<D>>().into(),
        }
    }

    fn reads() -> Vec<ResourceId> {
        vec![ResourceId::new::<Store<D>>()]
    }

    fn writes() -> Vec<ResourceId> {
        vec![]
    }
}

/// Grant mutable access to a (unnamed) store inside a System
pub struct WriteStore<'a, D>
where
    D: 'static,
{
    inner: Write<'a, Store<D>>,
}

impl<'a, D> Deref for WriteStore<'a, D>
where
    D: 'static,
{
    type Target = Store<D>;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a, D> DerefMut for WriteStore<'a, D>
where
    D: 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

impl<'a, D> SystemData<'a> for WriteStore<'a, D>
where
    D: 'static,
{
    fn setup(_: &mut shred::World) {}

    fn fetch(res: &'a shred::World) -> Self {
        WriteStore {
            inner: res.fetch_mut::<Store<D>>().into(),
        }
    }

    fn reads() -> Vec<ResourceId> {
        vec![]
    }

    fn writes() -> Vec<ResourceId> {
        vec![ResourceId::new::<Store<D>>()]
    }
}
