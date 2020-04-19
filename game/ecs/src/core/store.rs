use crate::core::arena::Arena;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak};
use std::{fmt, ops};

/// Data stored in the Store
pub trait Data {
    type Key: Clone + Send + Eq + Hash + fmt::Display;
    type LoadRequest: 'static + Send;
    type LoadResponse: 'static + Send;
    type UpdateContext;

    fn on_load(
        &mut self,
        context: &Self::UpdateContext,
        load_response: Self::LoadResponse,
    ) -> Option<Self::LoadRequest>;
}

pub trait FromKey: Data {
    fn from_key(key: &Self::Key) -> (Self, Option<<Self as Data>::LoadRequest>)
    where
        Self: Sized;
}

enum Key<D: Data> {
    Named(D::Key),
    Unnamed(usize),
}

impl<D: Data> Key<D> {
    fn is_named(&self) -> bool {
        match &self {
            Key::Named(_) => true,
            Key::Unnamed(_) => false,
        }
    }
}

impl<D: Data> PartialEq for Key<D> {
    fn eq(&self, other: &Self) -> bool {
        match (&self, &other) {
            (Key::Named(ref k1), Key::Named(ref k2)) => k1 == k2,
            (Key::Unnamed(ref k1), Key::Unnamed(ref k2)) => k1 == k2,
            _ => false,
        }
    }
}

impl<D: Data> Eq for Key<D> {}

impl<D: Data> Hash for Key<D> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self {
            Key::Named(ref k) => {
                "n".hash(state);
                k.hash(state);
            }
            Key::Unnamed(ref k) => {
                "u".hash(state);
                k.hash(state);
            }
        }
    }
}

impl<D: Data> Clone for Key<D> {
    fn clone(&self) -> Self {
        match &self {
            Key::Named(ref k) => Key::Named(k.clone()),
            Key::Unnamed(ref k) => Key::Unnamed(k.clone()),
        }
    }
}

impl<D: Data> fmt::Display for Key<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Key::Named(ref k) => write!(f, "Named({})", k),
            Key::Unnamed(ref id) => write!(f, "Unnamed({})", id),
        }
    }
}

/// Reference counted index to access stored items in O(1).
pub struct Index<D: Data>(*const Entry<D>);

unsafe impl<D: Data> Send for Index<D> {}
unsafe impl<D: Data> Sync for Index<D> {}

impl<D: Data> Index<D> {
    fn from_ref(entry: &Entry<D>) -> Index<D> {
        entry.ref_count.fetch_add(1, Ordering::Relaxed);
        Index(entry as *const _)
    }

    fn from_ptr(entry: *const Entry<D>) -> Index<D> {
        Self::from_ref(unsafe { &*entry })
    }

    unsafe fn entry(&self) -> &Entry<D> {
        &*self.0
    }

    unsafe fn entry_mut(&self) -> &mut Entry<D> {
        &mut *(self.0 as *mut _)
    }
}

impl<D: Data> PartialEq for Index<D> {
    fn eq(&self, e: &Self) -> bool {
        self.0 == e.0
    }
}

impl<D: Data> Clone for Index<D> {
    fn clone(&self) -> Index<D> {
        let entry = unsafe { self.entry() };
        entry.ref_count.fetch_add(1, Ordering::Relaxed);
        Index(self.0)
    }
}

impl<D: Data> Drop for Index<D> {
    fn drop(&mut self) {
        let entry = unsafe { self.entry() };
        entry.ref_count.fetch_sub(1, Ordering::Relaxed);
    }
}

/// An entry in the store.
struct Entry<D: Data> {
    ref_count: AtomicUsize,
    load_token: Arc<()>,
    value: D,
}

impl<D: Data> Entry<D> {
    fn has_reference(&self) -> bool {
        self.ref_count.load(Ordering::Relaxed) > 0
    }
}

/// Shared data that may have multiple readers (or a single writer)
/// The ready to be used resources are stored here those can be
/// used from multiple threads at the same time.
struct SharedData<D: Data> {
    entries: HashMap<Key<D>, (usize, *mut Entry<D>)>,
    load_requests: UnboundedSender<(D::LoadRequest, LoadContext<D>)>,
    load_responses: UnboundedReceiver<(D::LoadResponse, LoadContext<D>)>,
}

impl<D: Data> SharedData<D> {
    fn get(&self, k: &Key<D>) -> Option<Index<D>> {
        self.entries.get(k).map(|(_, ptr)| Index::from_ptr(*ptr))
    }
}

/// Shared data those requires exclusive access always.
/// The pending, new resourceas and memory managment related objects are stored here
/// those require explicit log for access.
struct ExclusiveData<D: Data> {
    arena: Arena<Entry<D>>,
    entries: HashMap<Key<D>, (usize, *mut Entry<D>)>,
    unnamed_id: usize,
    load_requests: UnboundedSender<(D::LoadRequest, LoadContext<D>)>,
}

impl<D: Data> ExclusiveData<D> {
    fn get(&self, k: &Key<D>) -> Option<Index<D>> {
        self.entries.get(k).map(|(_, ptr)| Index::from_ptr(*ptr))
    }

    /// Adds a new item to the store
    fn get_or_add<B: FnOnce(&Key<D>) -> (D, Option<D::LoadRequest>)>(&mut self, k: Key<D>, builder: B) -> Index<D> {
        let mut load_request = None;
        let entries = &mut self.entries;
        let arena = &mut self.arena;

        let (_, entry_ptr) = entries.entry(k.clone()).or_insert_with(|| {
            let (value, load) = builder(&k);
            let entry = Entry {
                ref_count: AtomicUsize::new(0),
                load_token: Arc::new(()),
                value,
            };
            let (id, entry) = arena.allocate(entry);
            if let Some(load) = load {
                load_request = Some((load, Arc::downgrade(&entry.load_token)))
            }
            (id, entry as *mut _)
        });

        let idx = Index::from_ptr(*entry_ptr);
        if let Some((load, load_token)) = load_request {
            log::debug!("Deferred loading [{}]", k);
            let context = LoadContext(load_token, *entry_ptr, k.clone());
            if let Err(err) = self.load_requests.unbounded_send((load, context)) {
                log::error!("Failed to send load task for [{}]: {:?}", k, err);
            }
        }
        idx
    }
}

/// The context of the loading operation to check cancelation.
pub struct LoadContext<D: Data>(Weak<()>, *mut Entry<D>, Key<D>);

unsafe impl<D> Send for LoadContext<D> where D: Data {}
unsafe impl<D> Sync for LoadContext<D> where D: Data {}

impl<D: Data> LoadContext<D> {
    pub fn is_canceled(&self) -> bool {
        self.0.upgrade().is_none()
    }
}

impl<D: Data> Clone for LoadContext<D> {
    fn clone(&self) -> Self {
        LoadContext(self.0.clone(), self.1, self.2.clone())
    }
}

pub trait DataLoader<D>: Send + Sync
where
    D: Data,
{
    fn load<'a>(
        &'a mut self,
        request: D::LoadRequest,
        context: LoadContext<D>,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<D::LoadResponse>> + Send + 'a>>;
}

struct NoDataLoader;

impl<D> DataLoader<D> for NoDataLoader
where
    D: Data,
{
    fn load<'a>(
        &'a mut self,
        _request: D::LoadRequest,
        _context: LoadContext<D>,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<D::LoadResponse>> + Send + 'a>> {
        Box::pin(futures::future::ready(None))
    }
}

pub struct StoreLoader<D>
where
    D: Data,
{
    requests: UnboundedReceiver<(D::LoadRequest, LoadContext<D>)>,
    responses: UnboundedSender<(D::LoadResponse, LoadContext<D>)>,
    data_loader: Box<dyn DataLoader<D>>,
}

unsafe impl<D> Send for StoreLoader<D> where D: Data {}

impl<D> StoreLoader<D>
where
    D: Data,
{
    pub async fn handle_one(&mut self) -> bool {
        log::info!("handle one");
        let (data, context) = match self.requests.next().await {
            Some((data, context)) => (data, context),
            None => {
                log::info!("Loader requests failed.");
                return false;
            }
        };

        if context.is_canceled() {
            return true;
        }
        let output = match self.data_loader.load(data, context.clone()).await {
            Some(output) => output,
            None => return true,
        };
        if context.is_canceled() {
            return true;
        }

        match self.responses.send((output, context)).await {
            Ok(_) => true,
            Err(err) => {
                log::info!("Loader response failed: {:?}", err);
                false
            }
        }
    }

    pub async fn run(mut self) {
        log::info!("run task");
        while self.handle_one().await {
            log::info!("wait");
        }
    }
}

/// Thread safe resource store.
/// While the store is locked for reading, no resource can be updated, but new one can be aquired:
/// - 1st the shared data is searched for an existing item (non-blocking)
/// - 2nd the mutex guarded shared data is used to find or create the resource. (blocking)
/// While the store is locked for write, resources are updated and released:
/// - 1st the items from the mutex guarded data are moved into the shared store.
/// - 2nd entries are updated based on the async loading queue
/// - 3nd on requests entries are dropped if there are no external references left. Dispite of
///   having a reference count for the sotred items, they are not reclaimed without an explicit request.
pub struct Store<D: Data> {
    shared: RwLock<SharedData<D>>,
    exclusive: Mutex<ExclusiveData<D>>,
}

unsafe impl<D: Data> Send for Store<D> {}
unsafe impl<D: Data> Sync for Store<D> {}

impl<D: Data> Store<D> {
    /// Create a new store without the loading pipeline.
    pub fn new(page_size: usize) -> Store<D> {
        Store::new_with_loader(page_size, NoDataLoader).0
    }

    /// Creates a new store and the async load handler.
    pub fn new_with_loader<L: 'static + DataLoader<D>>(page_size: usize, data_loader: L) -> (Store<D>, StoreLoader<D>) {
        let (req_send, req_recv) = mpsc::unbounded();
        let (resp_send, resp_recv) = mpsc::unbounded();
        (
            Store {
                shared: RwLock::new(SharedData {
                    entries: HashMap::new(),
                    load_requests: req_send.clone(),
                    load_responses: resp_recv,
                }),
                exclusive: Mutex::new(ExclusiveData {
                    arena: Arena::new(page_size),
                    entries: HashMap::new(),
                    unnamed_id: 0,
                    load_requests: req_send.clone(),
                }),
            },
            StoreLoader {
                requests: req_recv,
                responses: resp_send,
                data_loader: Box::new(data_loader),
            },
        )
    }

    /// Aquire read lock.
    pub fn try_read(&self) -> Option<ReadGuard<'_, D>> {
        let shared = self.shared.try_read().ok()?;
        Some(ReadGuard {
            shared,
            exclusive: &self.exclusive,
        })
    }

    /// Aquire read lock. In case of failure the function panics.
    pub fn read(&self) -> ReadGuard<'_, D> {
        self.try_read().unwrap()
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

    /// Aquire write lock. In case of failure the function panics.
    pub fn write(&mut self) -> WriteGuard<'_, D> {
        self.try_write().unwrap()
    }
}

impl<D> Store<D> where D: Data<LoadRequest = (), LoadResponse = ()> {}

impl<D: Data> Drop for Store<D> {
    fn drop(&mut self) {
        let shared = &mut *(self.shared.try_write().unwrap());
        let exclusive = &mut *(self.exclusive.lock().unwrap());
        let arena = &mut exclusive.arena;

        #[cfg(debug_assertions)]
        {
            let exclusive_entries = &mut exclusive.entries;
            for (k, (_, ptr)) in exclusive_entries {
                let entry = unsafe { &**ptr };
                debug_assert!(
                    !entry.has_reference(),
                    "Entry leak: [{}] is still referenced, shared",
                    k
                );
            }

            let shared_entries = &mut shared.entries;
            for (k, (_, ptr)) in shared_entries {
                let entry = unsafe { &**ptr };
                debug_assert!(
                    !entry.has_reference(),
                    "Entry leak: [{}] is still referenced, exclusive",
                    k
                );
            }
        }

        debug_assert!(arena.is_empty(), "Leaking entries");
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

        let k = Key::Named(k.clone());
        shared.get(&k).or_else(|| {
            if let Ok(exclusive) = exclusive.try_lock() {
                exclusive.get(&k)
            } else {
                None
            }
        })
    }

    pub fn named_get_blocking(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &self.exclusive;

        let k = Key::Named(k.clone());
        shared.get(&k).or_else(|| {
            let exclusive = exclusive.lock().unwrap();
            exclusive.get(&k)
        })
    }

    pub fn named_get_or_add_blocking(&mut self, k: &D::Key) -> Index<D>
    where
        D: FromKey,
    {
        let shared = &mut self.shared;
        let exclusive = &mut self.exclusive;

        let k = Key::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            let mut exclusive = exclusive.lock().unwrap();
            exclusive.get_or_add(k, |k| match &k {
                Key::Named(ref k) => <D as FromKey>::from_key(k),
                _ => unreachable!(),
            })
        })
    }

    pub fn add_blocking(&mut self, data: D) -> Index<D> {
        let mut exclusive = self.exclusive.lock().unwrap();

        exclusive.unnamed_id += 1;
        let k = Key::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_add(k, move |k| match &k {
            Key::Unnamed(_) => (data, None),
            _ => unreachable!(),
        })
    }

    pub fn add_blocking_with_load(&mut self, data: D, load: D::LoadRequest) -> Index<D> {
        let mut exclusive = self.exclusive.lock().unwrap();

        exclusive.unnamed_id += 1;
        let k = Key::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_add(k, move |k| match &k {
            Key::Unnamed(_) => (data, Some(load)),
            _ => unreachable!(),
        })
    }

    pub fn add_default_blocking(&mut self) -> Index<D>
    where
        D: Default,
    {
        self.add_blocking(<D as Default>::default())
    }

    pub fn at<'i: 'a>(&self, index: &'i Index<D>) -> &D {
        // To release/modify the indexed object from the container,
        // one have to get mutable reference to the store,
        // but that would contradict to the borrow checker.
        unsafe { &index.entry().value }
    }
}

impl<'a, 'i: 'a, D: 'a + Data> ops::Index<&'i Index<D>> for ReadGuard<'a, D> {
    type Output = D;

    fn index(&self, index: &'i Index<D>) -> &Self::Output {
        self.at(index)
    }
}

/// Guarded update access to a store
pub struct WriteGuard<'a, D: Data> {
    shared: RwLockWriteGuard<'a, SharedData<D>>,
    locked_exclusive: MutexGuard<'a, ExclusiveData<D>>,
}

impl<'a, D: 'a + Data> WriteGuard<'a, D> {
    pub fn named_get(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &self.locked_exclusive;

        let k = Key::Named(k.clone());
        exclusive.get(&k).or_else(|| shared.get(&k))
    }

    pub fn named_get_or_add(&mut self, k: &D::Key) -> Index<D>
    where
        D: FromKey,
    {
        let shared = &mut self.shared;
        let exclusive = &mut self.locked_exclusive;

        let k = Key::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            exclusive.get_or_add(k, |k| match &k {
                Key::Named(ref k) => <D as FromKey>::from_key(k),
                _ => unreachable!(),
            })
        })
    }

    pub fn add(&mut self, data: D) -> Index<D> {
        let exclusive = &mut self.locked_exclusive;

        exclusive.unnamed_id += 1;
        let k = Key::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_add(k, move |k| match &k {
            Key::Unnamed(_) => (data, None),
            _ => unreachable!(),
        })
    }

    pub fn add_with_load(&mut self, data: D, load: D::LoadRequest) -> Index<D> {
        let exclusive = &mut self.locked_exclusive;

        exclusive.unnamed_id += 1;
        let k = Key::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_add(k, move |k| match &k {
            Key::Unnamed(_) => (data, Some(load)),
            _ => unreachable!(),
        })
    }

    pub fn add_default(&mut self) -> Index<D>
    where
        D: Default,
    {
        self.add(<D as Default>::default())
    }

    /// Returns if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.locked_exclusive.entries.is_empty() && self.shared.entries.is_empty()
    }

    /// Move all new (pending) entries into the shared container
    pub fn finalize_requests(&mut self) {
        self.shared.entries.extend(&mut self.locked_exclusive.entries.drain());
    }

    pub fn update(&mut self, update_context: &D::UpdateContext) {
        loop {
            let (response, load_context) = match self.shared.load_responses.try_next() {
                Ok(Some((response, load_context))) => (response, load_context),
                _ => break,
            };

            if load_context.is_canceled() {
                continue;
            }

            #[cfg(debug_assertions)]
            {
                use std::ptr;
                let stored = {
                    let shared = &self.shared;
                    let exclusive = &self.locked_exclusive;
                    let k = &load_context.2;
                    exclusive
                        .entries
                        .get(k)
                        .or_else(|| shared.entries.get(k))
                        .map(|(_, p)| *p)
                        .unwrap_or(ptr::null_mut())
                };
                debug_assert!(
                    stored == load_context.1,
                    "Borrow checker error, entry was altered while token is still valid"
                );
            }

            let entry = unsafe { &mut *load_context.1 };
            if let Some(req) = entry.value.on_load(update_context, response) {
                if let Err(err) = self.shared.load_requests.unbounded_send((req, load_context)) {
                    log::error!("Failed send load task: {:?}", err);
                }
            }
        }
    }

    fn drain_unused_filtered_impl<F: FnMut(&mut D) -> bool>(
        arena: &mut Arena<Entry<D>>,
        entries: &mut HashMap<Key<D>, (usize, *mut Entry<D>)>,
        filter: &mut F,
    ) {
        entries.retain(|k, (id, ptr)| {
            let entry = unsafe { &mut **ptr };
            if !entry.has_reference() {
                if !k.is_named() || filter(&mut entry.value) {
                    log::debug!("Draining {}", k);
                    arena.deallocate(*id);
                    false
                } else {
                    true
                }
            } else {
                true
            }
        });
    }

    /// Drain unreferenced elements those fullfill the given predicate.
    /// In other words, remove all unreferenced entries such that f(&mut data) returns true.
    pub fn drain_unused_filtered<F: FnMut(&mut D) -> bool>(&mut self, mut filter: F) {
        let exclusive = &mut *self.locked_exclusive;
        Self::drain_unused_filtered_impl(&mut exclusive.arena, &mut self.shared.entries, &mut filter);
        Self::drain_unused_filtered_impl(&mut exclusive.arena, &mut exclusive.entries, &mut filter);
    }

    /// Drain all unreferenced items.
    pub fn drain_unused(&mut self) {
        self.drain_unused_filtered(|_| true)
    }

    pub fn at<'i: 'a>(&self, index: &'i Index<D>) -> &D {
        // To release/modify the indexed object from the container,
        // one have to get mutable reference to the store,
        // but that would contradict to the borrow checker.
        unsafe { &index.entry().value }
    }

    pub fn at_mut<'i: 'a>(&mut self, index: &'i Index<D>) -> &mut D {
        // To release/modify the indexed object from the container,
        // one have to get mutable reference to the store,
        // but that would contradict to the borrow checker.
        unsafe { &mut index.entry_mut().value }
    }
}

impl<'a, 'i: 'a, D: 'a + Data> ops::Index<&'i Index<D>> for WriteGuard<'a, D> {
    type Output = D;

    fn index(&self, index: &'i Index<D>) -> &Self::Output {
        self.at(index)
    }
}

impl<'a, 'i: 'a, D: 'a + Data> ops::IndexMut<&'i Index<D>> for WriteGuard<'a, D> {
    fn index_mut(&mut self, index: &'i Index<D>) -> &mut Self::Output {
        self.at_mut(index)
    }
}
