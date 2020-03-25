use crate::store::arena::PinnedArena;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::{fmt, ops};

/// Reference counted indexing of the store items in O(1).
pub struct Index<D>(*mut Entry<D>);

unsafe impl<D> Send for Index<D> {}
unsafe impl<D> Sync for Index<D> {}

impl<D> Index<D> {
    fn new(entry: *mut Entry<D>) -> Index<D> {
        assert!(!entry.is_null());
        unsafe { &(*entry).ref_count.fetch_add(1, Ordering::Relaxed) };
        Index(entry)
    }
}

impl<D> fmt::Debug for Index<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(!self.0.is_null());
        let rc = unsafe { &(*self.0).ref_count.load(Ordering::Relaxed) };
        write!(f, "Index({:p}, rc:{})", self.0, rc)
    }
}

impl<D> PartialEq for Index<D> {
    fn eq(&self, e: &Self) -> bool {
        assert!(!self.0.is_null());
        assert!(!e.0.is_null());
        self.0 == e.0
    }
}

impl<D> Clone for Index<D> {
    fn clone(&self) -> Index<D> {
        assert!(!self.0.is_null());
        unsafe { &(*self.0).ref_count.fetch_add(1, Ordering::Relaxed) };
        Index(self.0)
    }
}

impl<D> Drop for Index<D> {
    fn drop(&mut self) {
        assert!(!self.0.is_null());
        unsafe { &(*self.0).ref_count.fetch_sub(1, Ordering::Relaxed) };
    }
}

/// An entry in the store.
#[derive(Debug)]
struct Entry<D> {
    /// Number of active Index (number of references) to this entry
    ref_count: AtomicUsize,
    /// The stored data
    value: D,
}

// Store data that requires exclusive lock
struct SharedData<D> {
    resources: Vec<*mut Entry<D>>,
}

// D that requires exclusive lock
struct ExclusiveData<D> {
    arena: PinnedArena<Entry<D>>,
    requests: Vec<*mut Entry<D>>,
}

impl<D> ExclusiveData<D> {
    /// Adds a new item to the store
    fn add(&mut self, data: D) -> Index<D> {
        let entry = self.arena.allocate(Entry {
            ref_count: AtomicUsize::new(0),
            value: data,
        });
        let entry = entry as *mut Entry<D>;

        let index = Index::new(entry);
        self.requests.push(entry);
        index
    }
}

/// Thread safe resource store. Simmilar to the HashStore, but items can be aquired only by index,
/// no unique key is present and once all the indices are dropped, item cannot be retreaved from the store.
pub struct Store<D> {
    shared: RwLock<SharedData<D>>,
    exclusive: Mutex<ExclusiveData<D>>,
}

unsafe impl<D> Send for Store<D> {}
unsafe impl<D> Sync for Store<D> {}

impl<D> Store<D> {
    pub fn new() -> Store<D> {
        Store {
            shared: RwLock::new(SharedData { resources: Vec::new() }),
            exclusive: Mutex::new(ExclusiveData {
                arena: PinnedArena::new(),
                requests: Vec::new(),
            }),
        }
    }

    /// Creates a new store with memory allocated for at least capacity items
    pub fn new_with_capacity(_page_size: usize, capacity: usize) -> Store<D> {
        Store {
            shared: RwLock::new(SharedData {
                resources: Vec::with_capacity(capacity),
            }),
            exclusive: Mutex::new(ExclusiveData {
                arena: PinnedArena::new(), /*Arena::_with_capacity(page_size, capacity)*/
                requests: Vec::with_capacity(capacity),
            }),
        }
    }

    /// Aquire read lock.
    pub fn try_read(&self) -> Option<ReadGuard<'_, D>> {
        let shared = self.shared.try_read().ok()?;
        Some(ReadGuard {
            _shared: shared,
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

impl<D> Default for Store<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D> Drop for Store<D> {
    fn drop(&mut self) {
        let shared = &mut *(self.shared.write().unwrap());
        let exclusive = &mut *(self.exclusive.lock().unwrap());
        let arena = &mut exclusive.arena;
        let requests = &mut exclusive.requests;
        let resources = &mut shared.resources;

        resources.drain_filter(|&mut v| {
            let v = unsafe { &mut *v };
            assert!(v.ref_count.load(Ordering::Relaxed) == 0, "resource leak");
            arena.deallocate(v);
            true
        });

        requests.drain_filter(|&mut v| {
            let v = unsafe { &mut *v };
            assert!(v.ref_count.load(Ordering::Relaxed) == 0, "resource leak");
            arena.deallocate(v);
            true
        });

        assert!(resources.is_empty(), "Leaking resource");
        assert!(requests.is_empty(), "Leaking requests");
        assert!(arena.is_empty(), "Leaking arena, internal store error");
    }
}

/// Guarded read access to a store
pub struct ReadGuard<'a, D> {
    _shared: RwLockReadGuard<'a, SharedData<D>>,
    exclusive: &'a Mutex<ExclusiveData<D>>,
}

impl<'a, D: 'a> ReadGuard<'a, D> {
    pub fn add(&self, data: D) -> Index<D> {
        let mut exclusive = self.exclusive.lock().unwrap();
        exclusive.add(data)
    }

    /// Try to add the item to the store. On success the index is returned.
    /// If operation cannot be carried out immediatelly, data is returned back in the Error.
    pub fn try_add(&self, data: D) -> Result<Index<D>, D> {
        if let Ok(mut exclusive) = self.exclusive.try_lock() {
            Ok(exclusive.add(data))
        } else {
            Err(data)
        }
    }

    pub fn at(&self, index: &Index<D>) -> &D {
        assert!(!index.0.is_null(), "Indexing is invalid");
        let entry = unsafe { &(*index.0) };
        &entry.value
    }
}

impl<'a, 'i, D: 'a> ops::Index<&'i Index<D>> for ReadGuard<'a, D> {
    type Output = D;

    fn index(&self, index: &Index<D>) -> &Self::Output {
        self.at(index)
    }
}

/// Guarded update access to a store
pub struct WriteGuard<'a, D> {
    shared: RwLockWriteGuard<'a, SharedData<D>>,
    locked_exclusive: MutexGuard<'a, ExclusiveData<D>>,
}

impl<'a, D: 'a> WriteGuard<'a, D> {
    pub fn add(&mut self, data: D) -> Index<D> {
        self.locked_exclusive.add(data)
    }

    /// Returns if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.locked_exclusive.requests.is_empty() && self.shared.resources.is_empty()
    }

    /// Merges the requests into the "active" items
    pub fn finalize_requests(&mut self) {
        // Move all resources into the stored resources
        self.shared.resources.append(&mut self.locked_exclusive.requests);
    }

    fn drain_impl<F: FnMut(&mut D) -> bool>(
        arena: &mut PinnedArena<Entry<D>>,
        v: &mut Vec<*mut Entry<D>>,
        filter: &mut F,
    ) {
        v.drain_filter(|&mut e| {
            let e = unsafe { &mut *e };
            if e.ref_count.load(Ordering::Relaxed) == 0 {
                let drain = filter(&mut e.value);
                if drain {
                    arena.deallocate(e);
                }
                drain
            } else {
                false
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

impl<'a, 'i, D: 'a> ops::Index<&'i Index<D>> for WriteGuard<'a, D> {
    type Output = D;

    fn index(&self, index: &Index<D>) -> &Self::Output {
        self.at(index)
    }
}

impl<'a, 'i, D: 'a> ops::IndexMut<&'i Index<D>> for WriteGuard<'a, D> {
    fn index_mut(&mut self, index: &Index<D>) -> &mut Self::Output {
        self.at_mut(index)
    }
}

#[cfg(test)]
mod test {
    use super::Store;
    use std::sync::Arc;
    use std::{env, mem, thread};

    /// Test resource data
    struct TestData(String);

    impl TestData {
        fn new<S: Into<String>>(s: S) -> TestData {
            let string: String = s.into();
            log::trace!("creating '{}'", string);
            TestData(string.into())
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

        log::info!("request 0,1");
        {
            let store = store.try_read().unwrap();

            r0 = store.add(TestData::new("zero"));
            assert!(store[&r0].0 == "zero");

            r1 = store.add(TestData::new("one"));
            assert!(store[&r0].0 == "zero");
            assert!(store[&r1].0 == "one");
        }

        log::info!("request process");
        {
            let mut store = store.try_write().unwrap();
            store.finalize_requests();
        }

        log::info!("check 0,1, request 2");
        {
            let store = store.try_read().unwrap();
            assert!(store[&r0].0 == "zero");
            assert!(store[&r1].0 == "one");

            let r2 = store.add(TestData::new("two"));
            assert!(store[&r2].0 == "two");
        }

        log::info!("drop 2");
        {
            let mut store = store.try_write().unwrap();
            store.finalize_requests();
            store.drain_unused();
        }

        {
            let store = store.try_read().unwrap();
            assert!(store[&r0].0 == "zero");
            assert!(store[&r1].0 == "one");

            mem::drop(r1);
            assert!(store[&r0].0 == "zero");
        }

        log::info!("drop 1");
        {
            let mut store = store.try_write().unwrap();
            store.finalize_requests();
            store.drain_unused();
        }

        log::info!("drop 0");
        {
            mem::drop(r0);
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
                || env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "0".to_string()) == "1"
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
                    let store = store.try_read().unwrap();

                    // request 1
                    let r1 = store.add(TestData::new("one"));
                    assert!(store[&r1].0 == "one");

                    // request 100 + threadId
                    let r100 = store.add(TestData::new(format!("id: {}", 100 + i)));
                    assert!(store[&r100].0 == format!("id: {}", 100 + i));

                    for _ in 0..100 {
                        assert!(store[&r1].0 == "one");
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
