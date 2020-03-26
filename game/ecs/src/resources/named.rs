use crate::store::namedstore::Store as InnerStore;
use shred::{Read, ResourceId, SystemData, Write};
use std::ops::{Deref, DerefMut};

pub use crate::store::namedstore::{Data, Index, ReadGuard, WriteGuard};

/// A wrapper around [InnerStore](InnerStore) to make it more ergonomic for the world use
pub struct Store<D: Data> {
    inner: InnerStore<D>,
}

impl<D: Data> Store<D> {
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

impl<D: Data> Default for Store<D> {
    fn default() -> Self {
        Store::new()
    }
}

/// Grant immutable access to a named store inside a System
pub struct ReadNamedStore<'a, D>
where
    D: 'static + Data,
{
    inner: Read<'a, Store<D>>,
}

impl<'a, D> Deref for ReadNamedStore<'a, D>
where
    D: 'static + Data,
{
    type Target = Store<D>;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a, D> SystemData<'a> for ReadNamedStore<'a, D>
where
    D: 'static + Data,
{
    fn setup(_: &mut shred::World) {}

    fn fetch(res: &'a shred::World) -> Self {
        ReadNamedStore {
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

/// Grant mutable access to a named store inside a System
pub struct WriteNamedStore<'a, D>
where
    D: 'static + Data,
{
    inner: Write<'a, Store<D>>,
}

impl<'a, D> Deref for WriteNamedStore<'a, D>
where
    D: 'static + Data,
{
    type Target = Store<D>;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<'a, D> DerefMut for WriteNamedStore<'a, D>
where
    D: 'static + Data,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.deref_mut()
    }
}

impl<'a, D> SystemData<'a> for WriteNamedStore<'a, D>
where
    D: 'static + Data,
{
    fn setup(_: &mut shred::World) {}

    fn fetch(res: &'a shred::World) -> Self {
        WriteNamedStore {
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
