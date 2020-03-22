use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::ops;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::store::arena::PinnedArena;

/// Data stored in the Store
pub trait Data {
    type Key: Clone + Send + Eq + Hash + fmt::Debug;

    fn from_key(key: Self::Key) -> Self
    where
        Self: Sized;
}

/// Reference counted index to access stored items in O(1).
/// Eventough index has a (mutable) reference to the data, to aquire it, a properly locked
/// store is required. The entry pointer is private, implementation detail that shall never
///  be made public and used to speed up the storage indexing.
pub struct Index<D: Data>(*mut Entry<D>);

unsafe impl<D: Data> Send for Index<D> {}
unsafe impl<D: Data> Sync for Index<D> {}

impl<D: Data> Index<D> {
    fn new(entry: *mut Entry<D>) -> Index<D> {
        assert!(!entry.is_null());
        unsafe { &(*entry).ref_count.fetch_add(1, Ordering::Relaxed) };
        Index(entry)
    }
}

impl<D: Data> fmt::Debug for Index<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(!self.0.is_null());
        // the referneced data cannot be debug formatted as
        // 1. it is not required in the trait
        // 2. and it might be modified by another thread (see the Index struct).
        let rc = unsafe { &(*self.0).ref_count.load(Ordering::Relaxed) };
        write!(f, "Index({:p}, rc:{})", self.0, rc)
    }
}

impl<D: Data> PartialEq for Index<D> {
    fn eq(&self, e: &Self) -> bool {
        assert!(!self.0.is_null());
        assert!(!e.0.is_null());
        self.0 == e.0
    }
}

impl<D: Data> Clone for Index<D> {
    fn clone(&self) -> Index<D> {
        assert!(!self.0.is_null());
        unsafe { &(*self.0).ref_count.fetch_add(1, Ordering::Relaxed) };
        Index(self.0)
    }
}

impl<D: Data> Drop for Index<D> {
    fn drop(&mut self) {
        assert!(!self.0.is_null());
        unsafe { &(*self.0).ref_count.fetch_sub(1, Ordering::Relaxed) };
    }
}

/// An entry in the store.
#[derive(Debug)]
struct Entry<D: Data> {
    /// Number of active Index (number of references) to this entry
    ref_count: AtomicUsize,

    /// The stored data
    value: D,
}

// Shared data storing the new (pending) items
struct SharedData<D: Data> {
    resources: HashMap<D::Key, *mut Entry<D>>,
}

impl<D: Data> SharedData<D> {
    fn get(&self, k: &D::Key) -> Option<Index<D>> {
        self.resources.get(k).map(|&v| Index::new(v))
    }
}

// Shared data storing the active items those require exclusive access to be updated.
struct ExclusiveData<D: Data> {
    arena: PinnedArena<Entry<D>>,
    requests: HashMap<D::Key, *mut Entry<D>>,
}

impl<D: Data> ExclusiveData<D> {
    fn get(&self, k: &D::Key) -> Option<Index<D>> {
        self.requests.get(k).map(|&v| Index::new(v))
    }

    /// Adds a new item to the store
    fn get_or_add(&mut self, k: &D::Key) -> Index<D> {
        let arena = &mut self.arena;
        let entry = self.requests.entry(k.clone()).or_insert_with(|| {
            let new_entry = arena.allocate(Entry {
                ref_count: AtomicUsize::new(0),
                value: <D as Data>::from_key(k.clone()),
            });
            new_entry as *mut Entry<D>
        });

        Index::new(*entry)
    }
}

/// Thread safe resource store.
/// While the store is locked for reading, no resource can be updated, but new one can be created
/// with a two phase storage policy
/// - first the shared data is searched for an existing items (non-blocking)
/// - the secondary mutex guarded data is used if the item's been already added (blocking)
/// - no data is released (dropped), dispite of having a reference count of zero.
/// When the store is write locked
/// - the items from the secondary storage are moved into the primary one
/// - resources can be updated
/// - resources can be dropped if the reference count is zero
pub struct Store<D: Data> {
    shared: RwLock<SharedData<D>>,
    exclusive: Mutex<ExclusiveData<D>>,
}

unsafe impl<D: Data> Send for Store<D> {}

unsafe impl<D: Data> Sync for Store<D> {}

impl<D: Data> Store<D> {
    pub fn new() -> Store<D> {
        Store {
            shared: RwLock::new(SharedData {
                resources: HashMap::new(),
            }),
            exclusive: Mutex::new(ExclusiveData {
                arena: PinnedArena::new(),
                requests: HashMap::new(),
            }),
        }
    }

    /// Creates a new store with memory allocated for at least capacity items
    pub fn new_with_capacity(_page_size: usize, capacity: usize) -> Store<D> {
        Store {
            shared: RwLock::new(SharedData {
                resources: HashMap::with_capacity(capacity),
            }),
            exclusive: Mutex::new(ExclusiveData {
                arena: PinnedArena::new(), /*Arena::_with_capacity(page_size, capacity)*/
                requests: HashMap::with_capacity(capacity),
            }),
        }
    }

    /// Aquire read lock.
    pub fn try_read(&self) -> Option<ReadGuard<'_, D>> {
        let shared = self.shared.try_read().ok()?;
        Some(ReadGuard {
            shared,
            exclusive: &self.exclusive,
        })
    }

    /// Try to aquire write lock.
    pub fn try_write(&self) -> Option<WriteGuard<'_, D>> {
        let shared = self.shared.try_write().ok()?;
        let locked_exclusive = self.exclusive.lock().ok()?;
        Some(WriteGuard {
            shared,
            locked_exclusive,
        })
    }
}

impl<D: Data> Default for Store<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: Data> Drop for Store<D> {
    fn drop(&mut self) {
        let shared = &mut *(self.shared.try_write().unwrap());
        let exclusive = &mut *(self.exclusive.lock().unwrap());
        let arena = &mut exclusive.arena;
        let requests = &mut exclusive.requests;
        let resources = &mut shared.resources;

        resources.retain(|_, &mut v| {
            let v = unsafe { &mut *v };
            assert!(v.ref_count.load(Ordering::Relaxed) == 0, "resource leak");
            arena.deallocate(v);
            false
        });

        requests.retain(|_, &mut v| {
            let v = unsafe { &mut *v };
            assert!(v.ref_count.load(Ordering::Relaxed) == 0, "resource leak");
            arena.deallocate(v);
            false
        });

        assert!(resources.is_empty(), "Leaking resource");
        assert!(requests.is_empty(), "Leaking requests");
        assert!(arena.is_empty(), "Leaking arena, internal store error");
    }
}

/// Guarded read access to a store
pub struct ReadGuard<'a, D: Data> {
    shared: RwLockReadGuard<'a, SharedData<D>>,
    exclusive: &'a Mutex<ExclusiveData<D>>,
}

impl<'a, D: 'a + Data> ReadGuard<'a, D> {
    /// Try to get the index of a resource by the key.
    /// If the global container is not accessible (ex. update is in progress),
    /// a null index is returned.
    pub fn try_get(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &self.exclusive;

        shared.get(k).or_else(|| {
            if let Ok(exclusive) = exclusive.try_lock() {
                exclusive.get(k)
            } else {
                None
            }
        })
    }

    pub fn get_blocking(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &self.exclusive;

        shared.get(k).or_else(|| {
            let exclusive = exclusive.lock().unwrap();
            exclusive.get(k)
        })
    }

    pub fn get_or_add_blocking(&mut self, k: &D::Key) -> Index<D> {
        let shared = &mut self.shared;
        let exclusive = &mut self.exclusive;

        shared.get(k).unwrap_or_else(|| {
            let mut exclusive = exclusive.lock().unwrap();
            exclusive.get_or_add(&k)
        })
    }

    pub fn at(&self, index: &Index<D>) -> &D {
        assert!(!index.0.is_null(), "Indexing is invalid");
        let entry = unsafe { &(*index.0) };
        &entry.value
    }
}

impl<'a, 'i, D: 'a + Data> ops::Index<&'i Index<D>> for ReadGuard<'a, D> {
    type Output = D;

    fn index(&self, index: &Index<D>) -> &Self::Output {
        self.at(index)
    }
}

/// Guarded update access to a store
pub struct WriteGuard<'a, D: Data> {
    shared: RwLockWriteGuard<'a, SharedData<D>>,
    locked_exclusive: MutexGuard<'a, ExclusiveData<D>>,
}

impl<'a, D: 'a + Data> WriteGuard<'a, D> {
    pub fn get(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &self.locked_exclusive;

        exclusive.get(k).or_else(|| shared.get(k))
    }

    pub fn get_or_add(&mut self, k: &D::Key) -> Index<D> {
        let shared = &mut self.shared;
        let exclusive = &mut self.locked_exclusive;

        shared.get(k).unwrap_or_else(|| exclusive.get_or_add(&k))
    }

    /// Returns if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.locked_exclusive.requests.is_empty() && self.shared.resources.is_empty()
    }

    /// Move all new (pending) resources into the active resources
    pub fn finalize_requests(&mut self) {
        self.shared
            .resources
            .extend(&mut self.locked_exclusive.requests.drain());
    }

    fn drain_impl<F: FnMut(&mut D) -> bool>(
        arena: &mut PinnedArena<Entry<D>>,
        v: &mut HashMap<D::Key, *mut Entry<D>>,
        filter: &mut F,
    ) {
        v.retain(|_k, &mut e| {
            let e = unsafe { &mut *e };
            if e.ref_count.load(Ordering::Relaxed) == 0 {
                let drain = filter(&mut e.value);
                if drain {
                    arena.deallocate(e);
                }
                !drain
            } else {
                true
            }
        });
    }

    /// Drain unreferenced elements those specified by the predicate.
    /// In other words, remove all unreferenced resources such that f(&mut data) returns true.
    pub fn drain_unused_filtered<F: FnMut(&mut D) -> bool>(&mut self, mut filter: F) {
        let exclusive = &mut *self.locked_exclusive;
        Self::drain_impl(&mut exclusive.arena, &mut self.shared.resources, &mut filter);
        Self::drain_impl(&mut exclusive.arena, &mut exclusive.requests, &mut filter);
    }

    /// Drain all unreferenced items. Only the referenced items are kept in the store.
    pub fn drain_unused(&mut self) {
        self.drain_unused_filtered(|_| true)
    }

    pub fn at(&self, index: &Index<D>) -> &D {
        assert!(!index.0.is_null(), "Indexing is invalid");
        let entry = unsafe { &(*index.0) };
        &entry.value
    }

    pub fn at_mut(&mut self, index: &Index<D>) -> &mut D {
        assert!(!index.0.is_null(), "Indexing is invalid");
        let entry = unsafe { &mut (*index.0) };
        &mut entry.value
    }
}

impl<'a, 'i, D: 'a + Data> ops::Index<&'i Index<D>> for WriteGuard<'a, D> {
    type Output = D;

    fn index(&self, index: &Index<D>) -> &Self::Output {
        self.at(index)
    }
}

impl<'a, 'i, D: 'a + Data> ops::IndexMut<&'i Index<D>> for WriteGuard<'a, D> {
    fn index_mut(&mut self, index: &Index<D>) -> &mut Self::Output {
        self.at_mut(index)
    }
}

#[cfg(test)]
mod test {
    use super::{Data, Store};
    use std::sync::Arc;
    use std::{env, mem, thread};

    /// Resource id for test data
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    struct TestDataId(u32);

    /// Test resource data
    struct TestData(String);

    impl Data for TestData {
        type Key = TestDataId;

        fn from_key(k: TestDataId) -> TestData {
            Self::new(format!("id: {}", k.0))
        }
    }

    impl TestData {
        fn new(s: String) -> TestData {
            log::trace!("creating '{}'", s);
            TestData(s)
        }
    }

    impl Drop for TestData {
        fn drop(&mut self) {
            log::trace!("dropping '{}'", self.0);
        }
    }

    #[test]
    fn simple_single_threaded() {
        let store = Store::<TestData>::new();
        let r0; // = TestRef::none();
        let r1; // = TestRef::none();

        log::debug!("request 0,1");
        {
            let mut store = store.try_read().unwrap();
            assert!(store.get_blocking(&TestDataId(0)) == None);

            r0 = store.get_or_add_blocking(&TestDataId(0));
            assert!(store[&r0].0 == format!("id: {}", 0));

            r1 = store.get_or_add_blocking(&TestDataId(1));
            assert!(store[&r1].0 == format!("id: {}", 1));
            let r11 = store.get_blocking(&TestDataId(1)).unwrap();
            assert!(store[&r11].0 == format!("id: {}", 1));
            assert!(r11 == r1);
            let r12 = store.get_or_add_blocking(&TestDataId(1));
            assert!(store[&r12].0 == format!("id: {}", 1));
            assert!(r12 == r1);
        }

        log::debug!("request process");
        {
            let mut store = store.try_write().unwrap();
            store.finalize_requests();
        }

        log::debug!("check 0,1, request 2");
        {
            let mut store = store.try_read().unwrap();
            assert!(store[&r0].0 == format!("id: {}", 0));
            assert!(store.get_blocking(&TestDataId(0)).unwrap() == r0);
            assert!(store[&r1].0 == format!("id: {}", 1));
            assert!(store.get_blocking(&TestDataId(1)).unwrap() == r1);

            let r2 = store.get_or_add_blocking(&TestDataId(2));
            assert!(store[&r2].0 == format!("id: {}", 2));
        }

        log::debug!("drop 2");
        {
            let mut store = store.try_write().unwrap();
            store.finalize_requests();
            store.drain_unused();
        }

        {
            let store = store.try_read().unwrap();
            assert!(store.get_blocking(&TestDataId(2)) == None);

            assert!(store[&r0].0 == format!("id: {}", 0));
            assert!(store.get_blocking(&TestDataId(0)).unwrap() == r0);
            assert!(store[&r1].0 == format!("id: {}", 1));
            assert!(store.get_blocking(&TestDataId(1)).unwrap() == r1);

            mem::drop(r1);
            // check that store is not yet modified
            assert!(store[&store.get_blocking(&TestDataId(1)).unwrap()].0 == format!("id: {}", 1));
            //info!("{:?}", r1);
        }

        log::debug!("drop 1");
        {
            let mut store = store.try_write().unwrap();
            store.finalize_requests();
            store.drain_unused();
        }

        {
            let store = store.try_read().unwrap();
            assert!(store[&r0].0 == format!("id: {}", 0));
            assert!(store.get_blocking(&TestDataId(0)).unwrap() == r0);
            assert!(store.get_blocking(&TestDataId(1)) == None);
            assert!(store.get_blocking(&TestDataId(2)) == None);

            mem::drop(r0);
            // check that store is not modified yet
            assert!(store[&store.get_blocking(&TestDataId(0)).unwrap()].0 == format!("id: {}", 0));
        }

        log::debug!("drop 0");
        {
            let mut store = store.try_write().unwrap();
            store.finalize_requests();
            store.drain_unused();
            assert!(store.is_empty());
        }
    }

    #[test]
    fn simple_multi_threaded() {
        assert!(
            env::args().any(|a| a == "--test-threads=1")
                || env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "0".to_string()) == "1",
        );

        let store = Store::<TestData>::new();
        let store = Arc::new(store);

        const ITER: u32 = 10;

        // request from multiple threads
        {
            let mut tp = vec![];
            for i in 0..ITER {
                let store = store.clone();
                tp.push(thread::spawn(move || {
                    let mut store = store.try_read().unwrap();
                    assert!(store.get_blocking(&TestDataId(0)) == None);

                    // request 1
                    let r1 = store.get_or_add_blocking(&TestDataId(1));
                    assert!(store[&r1].0 == format!("id: {}", 1));

                    // request 100 + threadId
                    let r100 = store.get_or_add_blocking(&TestDataId(100 + i));
                    assert!(store[&r100].0 == format!("id: {}", 100 + i));

                    for _ in 0..100 {
                        assert!(store[&r1].0 == format!("id: {}", 1));
                        assert!(store[&r100].0 == format!("id: {}", 100 + i));
                    }
                }));
            }
            for t in tp.drain(..) {
                t.join().unwrap();
            }
        }

        log::info!("request process");
        {
            let mut store = store.try_write().unwrap();
            store.finalize_requests();
            // no drain
        }

        // check after process
        {
            let mut tp = vec![];
            for i in 0..ITER {
                let store = store.clone();
                tp.push(thread::spawn(move || {
                    let store = store.try_read().unwrap();
                    assert!(store.get_blocking(&TestDataId(0)) == None);

                    // get 1
                    let r1 = store.get_blocking(&TestDataId(1)).unwrap();
                    assert!(store[&r1].0 == format!("id: {}", 1));

                    // get 100 + threadId
                    let r100 = store.get_blocking(&TestDataId(100 + i)).unwrap();
                    assert!(store[&r100].0 == format!("id: {}", 100 + i));
                }));
            }
            for t in tp.drain(..) {
                t.join().unwrap();
            }
        }

        log::info!("drain");
        {
            let mut store = store.try_write().unwrap();
            store.finalize_requests();
            store.drain_unused();
            // no drain
        }

        // check after drain
        {
            let mut tp = vec![];
            for i in 0..ITER {
                let store = store.clone();
                tp.push(thread::spawn(move || {
                    let store = store.try_read().unwrap();
                    assert!(store.get_blocking(&TestDataId(0)) == None);

                    // get 1
                    assert!(store.get_blocking(&TestDataId(1)) == None);

                    // get 100 + threadId
                    assert!(store.get_blocking(&TestDataId(100 + i)) == None);
                }));
            }
            for t in tp.drain(..) {
                t.join().unwrap();
            }
        }
    }

    #[test]
    fn check_lock() {
        assert!(
            env::args().any(|a| a == "--test-threads=1")
                || env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "0".to_string()) == "1"
        );

        use std::mem;
        use std::panic;

        panic::set_hook(Box::new(|_info| { /*println!("panic: {:?}", _info);*/ }));

        {
            let store = Store::<TestData>::new();
            assert!(panic::catch_unwind(|| {
                let w = store.try_write().unwrap();
                let r = store.try_read().unwrap();
                drop(r);
                drop(w);
            })
            .is_err());
            mem::forget(store);
        }

        {
            let store = Store::<TestData>::new();
            assert!(panic::catch_unwind(|| {
                let r = store.try_read().unwrap();
                let w = store.try_write().unwrap();
                drop(w);
                drop(r);
            })
            .is_err());
            mem::forget(store);
        }

        {
            let store = Store::<TestData>::new();
            assert!(panic::catch_unwind(|| {
                let w1 = store.try_write().unwrap();
                let w2 = store.try_write().unwrap();
                drop(w2);
                drop(w1);
            })
            .is_err());
            mem::forget(store);
        }

        let _ = panic::take_hook();
    }
}
