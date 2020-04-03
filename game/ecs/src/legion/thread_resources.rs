use downcast_rs::{impl_downcast, Downcast};
use fxhash::FxHashMap;
use std::{
    any::TypeId,
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

#[cfg(not(feature = "ffi"))]
/// A type ID identifying a component type.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct ThreadResourceTypeId(TypeId);

#[cfg(not(feature = "ffi"))]
impl ThreadResourceTypeId {
    /// Gets the component type ID that represents type `T`.
    pub fn of<T: ThreadResource>() -> Self {
        Self(TypeId::of::<T>())
    }
}

#[cfg(feature = "ffi")]
/// A type ID identifying a component type.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct ThreadResourceTypeId(TypeId, u32);

#[cfg(feature = "ffi")]
impl ThreadResourceTypeId {
    /// Gets the component type ID that represents type `T`.
    pub fn of<T: ThreadResource>() -> Self {
        Self(TypeId::of::<T>(), 0)
    }
}

/// Blanket trait for thread local resource types.
pub trait ThreadResource: 'static + Downcast {}
impl<T> ThreadResource for T where T: 'static {}
impl_downcast!(ThreadResource);

/// Ergonomic wrapper type which contains a `Ref` type.
pub struct Fetch<'a, T: 'a + ThreadResource> {
    inner: Ref<'a, Box<dyn ThreadResource>>,
    _marker: PhantomData<T>,
}
impl<'a, T: ThreadResource> Deref for Fetch<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner
            .downcast_ref::<T>()
            .unwrap_or_else(|| panic!("Unable to downcast the resource!: {}", std::any::type_name::<T>()))
    }
}

impl<'a, T: 'a + ThreadResource + std::fmt::Debug> std::fmt::Debug for Fetch<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

/// Ergonomic wrapper type which contains a `RefMut` type.
pub struct FetchMut<'a, T: ThreadResource> {
    inner: RefMut<'a, Box<dyn ThreadResource>>,
    _marker: PhantomData<T>,
}
impl<'a, T: 'a + ThreadResource> Deref for FetchMut<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner
            .downcast_ref::<T>()
            .unwrap_or_else(|| panic!("Unable to downcast the resource!: {}", std::any::type_name::<T>()))
    }
}

impl<'a, T: 'a + ThreadResource> DerefMut for FetchMut<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        self.inner
            .downcast_mut::<T>()
            .unwrap_or_else(|| panic!("Unable to downcast the resource!: {}", std::any::type_name::<T>()))
    }
}

impl<'a, T: 'a + ThreadResource + std::fmt::Debug> std::fmt::Debug for FetchMut<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

/// ThreadResources container. This container stores its underlying resources in a `FxHashMap` keyed on
/// `ThreadResourceTypeId`. This means that the ID's used in this storage will not persist between recompiles.
#[derive(Default)]
pub struct ThreadResources {
    storage: FxHashMap<ThreadResourceTypeId, RefCell<Box<dyn ThreadResource>>>,
}

impl ThreadResources {
    /// Returns `true` if type `T` exists in the store. Otherwise, returns `false`
    pub fn contains<T: ThreadResource>(&self) -> bool {
        self.storage.contains_key(&ThreadResourceTypeId::of::<T>())
    }

    /// Inserts the instance of `T` into the store. If the type already exists, it will be silently
    /// overwritten. If you would like to retain the instance of the resource that already exists,
    /// call `remove` first to retrieve it.
    pub fn insert<T: ThreadResource>(&mut self, value: T) {
        self.storage
            .insert(ThreadResourceTypeId::of::<T>(), RefCell::new(Box::new(value)));
    }

    /// Removes the type `T` from this store if it exists.
    ///
    /// # Returns
    /// If the type `T` was stored, the inner instance of `T is returned. Otherwise, `None`
    pub fn remove<T: ThreadResource>(&mut self) -> Option<T> {
        Some(
            *self
                .storage
                .remove(&ThreadResourceTypeId::of::<T>())?
                .into_inner()
                .downcast::<T>()
                .ok()?,
        )
    }

    /// Retrieve an immutable reference to  `T` from the store if it exists. Otherwise, return `None`
    pub fn get<T: ThreadResource>(&self) -> Option<Fetch<'_, T>> {
        Some(Fetch {
            inner: self.storage.get(&ThreadResourceTypeId::of::<T>())?.borrow(),
            _marker: Default::default(),
        })
    }

    /// Retrieve a mutable reference to  `T` from the store if it exists. Otherwise, return `None`
    pub fn get_mut<T: ThreadResource>(&self) -> Option<FetchMut<'_, T>> {
        Some(FetchMut {
            inner: self.storage.get(&ThreadResourceTypeId::of::<T>())?.borrow_mut(),
            _marker: Default::default(),
        })
    }

    /// Attempts to retrieve an immutable reference to `T` from the store. If it does not exist,
    /// the closure `f` is called to construct the object and it is then inserted into the store.
    pub fn get_or_insert_with<T: ThreadResource, F: FnOnce() -> T>(&mut self, f: F) -> Option<Fetch<'_, T>> {
        self.get_or_insert((f)())
    }

    /// Attempts to retrieve a mutable reference to `T` from the store. If it does not exist,
    /// the closure `f` is called to construct the object and it is then inserted into the store.
    pub fn get_mut_or_insert_with<T: ThreadResource, F: FnOnce() -> T>(&mut self, f: F) -> Option<FetchMut<'_, T>> {
        self.get_mut_or_insert((f)())
    }

    /// Attempts to retrieve an immutable reference to `T` from the store. If it does not exist,
    /// the provided value is inserted and then a reference to it is returned.
    pub fn get_or_insert<T: ThreadResource>(&mut self, value: T) -> Option<Fetch<'_, T>> {
        Some(Fetch {
            inner: self
                .storage
                .entry(ThreadResourceTypeId::of::<T>())
                .or_insert_with(|| RefCell::new(Box::new(value)))
                .borrow(),
            _marker: Default::default(),
        })
    }

    /// Attempts to retrieve a mutable reference to `T` from the store. If it does not exist,
    /// the provided value is inserted and then a reference to it is returned.
    pub fn get_mut_or_insert<T: ThreadResource>(&mut self, value: T) -> Option<FetchMut<'_, T>> {
        Some(FetchMut {
            inner: self
                .storage
                .entry(ThreadResourceTypeId::of::<T>())
                .or_insert_with(|| RefCell::new(Box::new(value)))
                .borrow_mut(),
            _marker: Default::default(),
        })
    }

    /// Attempts to retrieve an immutable reference to `T` from the store. If it does not exist,
    /// the default constructor for `T` is called.
    ///
    /// `T` must implement `Default` for this method.
    pub fn get_or_default<T: ThreadResource + Default>(&mut self) -> Option<Fetch<'_, T>> {
        Some(Fetch {
            inner: self
                .storage
                .entry(ThreadResourceTypeId::of::<T>())
                .or_insert_with(|| RefCell::new(Box::new(T::default())))
                .borrow(),
            _marker: Default::default(),
        })
    }

    /// Attempts to retrieve a mutable reference to `T` from the store. If it does not exist,
    /// the default constructor for `T` is called.
    ///
    /// `T` must implement `Default` for this method.
    pub fn get_mut_or_default<T: ThreadResource + Default>(&mut self) -> Option<FetchMut<'_, T>> {
        Some(FetchMut {
            inner: self
                .storage
                .entry(ThreadResourceTypeId::of::<T>())
                .or_insert_with(|| RefCell::new(Box::new(T::default())))
                .borrow_mut(),
            _marker: Default::default(),
        })
    }

    /// Performs merging of two resource storages, which occurs during a world merge.
    /// This merge will retain any already-existant resources in the local world, while moving any
    /// new resources from the source world into this one, consuming the resources.
    pub fn merge(&mut self, mut other: ThreadResources) {
        // Merge resources, retaining our local ones but moving in any non-existant ones
        for resource in other.storage.drain() {
            self.storage.entry(resource.0).or_insert(resource.1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_read_write_test() {
        let _ = tracing_subscriber::fmt::try_init();

        struct TestOne {
            value: String,
        }

        struct TestTwo {
            value: String,
        }

        let mut resources = ThreadResources::default();
        resources.insert(TestOne {
            value: "poop".to_string(),
        });

        resources.insert(TestTwo {
            value: "balls".to_string(),
        });

        assert_eq!(resources.get::<TestOne>().unwrap().value, "poop");
        assert_eq!(resources.get::<TestTwo>().unwrap().value, "balls");

        // test re-ownership
        let owned = resources.remove::<TestTwo>();
        assert_eq!(owned.unwrap().value, "balls")
    }
}
