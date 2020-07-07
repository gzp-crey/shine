use crate::core::arena::Arena;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak};
use std::{fmt, ptr};

/// Trait for the data stored in a Store
pub trait Data: Sized {
    type Key: Clone + Send + Eq + Hash + fmt::Debug;
}

/// Trait to construct data from a key
pub trait FromKey: Data {
    fn from_key(key: &Self::Key) -> Self
    where
        Self: Sized;
}

pub trait Load: Data {
    type LoadRequest: 'static + Send;
    type LoadResponse: 'static + Send;
}

/// Trait to load data from
pub trait OnLoad<'l>: Load {
    type LoadContext: 'l;
    type UpdateContext: 'l;

    fn start_load(&self, load_context: Self::LoadContext, load_token: LoadToken<Self>)
    where
        Self: Sized;

    fn on_loaded(&mut self, update_context: Self::UpdateContext, load_response: Self::LoadResponse)
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

impl<D: Data> fmt::Debug for Key<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Key::Named(ref k) => f.debug_tuple("Named").field(&k).finish(),
            Key::Unnamed(ref id) => f.debug_tuple("Unnamed").field(&id).finish(),
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

    unsafe fn from_ptr(entry: *const Entry<D>) -> Index<D> {
        Self::from_ref(&*entry)
    }

    unsafe fn entry(&self) -> &Entry<D> {
        &*self.0
    }

    #[allow(clippy::mut_from_ref)]
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

/// A token to test the cancelation of loading operations.
pub struct LoadToken<D: Data>(Weak<()>, *mut Entry<D>, Key<D>);

unsafe impl<D: Data> Send for LoadToken<D> {}
unsafe impl<D: Data> Sync for LoadToken<D> {}

impl<D: Data> LoadToken<D> {
    pub fn is_canceled(&self) -> bool {
        self.0.upgrade().is_none()
    }
}

impl<D: Data> Clone for LoadToken<D> {
    fn clone(&self) -> Self {
        LoadToken(self.0.clone(), self.1, self.2.clone())
    }
}

/// Shared data with multiple reador or single writer access.
/// This is the main data storage tho ensure no resources are altered during use.
struct SharedData<D: Data> {
    entries: HashMap<Key<D>, (usize, *mut Entry<D>)>,
}

impl<D: Data> SharedData<D> {
    fn get(&self, k: &Key<D>) -> Option<Index<D>> {
        self.entries.get(k).map(|(_, ptr)| unsafe { Index::from_ptr(*ptr) })
    }
}

/// Shared data with exclusive access always.
/// This is a transient area for the newly created resources.
struct ExclusiveData<D: Data> {
    arena: Arena<Entry<D>>,
    entries: HashMap<Key<D>, (usize, *mut Entry<D>)>,
    unnamed_id: usize,
}

impl<D: Data> ExclusiveData<D> {
    fn get(&self, k: &Key<D>) -> Option<Index<D>> {
        self.entries.get(k).map(|(_, ptr)| unsafe { Index::from_ptr(*ptr) })
    }

    /// Get or create a new item.
    fn get_or_create<B, L>(&mut self, k: Key<D>, build: B, post_build: L) -> Index<D>
    where
        B: FnOnce(&Key<D>) -> D,
        L: FnOnce(&Key<D>, &mut Entry<D>),
    {
        let entries = &mut self.entries;
        let arena = &mut self.arena;

        let (_, entry_ptr) = entries.entry(k.clone()).or_insert_with(|| {
            let value = build(&k);
            let entry = Entry {
                ref_count: AtomicUsize::new(0),
                load_token: Arc::new(()),
                value,
            };
            let (id, mut entry) = arena.allocate(entry);
            post_build(&k, &mut entry);
            (id, entry as *mut _)
        });

        unsafe { Index::from_ptr(*entry_ptr) }
    }

    /// Get or load a new item
    fn get_or_add<B: FnOnce(&Key<D>) -> D>(&mut self, k: Key<D>, builder: B) -> Index<D>
    where
        D: FromKey,
    {
        self.get_or_create(k, builder, |_, _| {})
    }

    /// Get or load a new item
    fn get_or_load<'l, B: FnOnce(&Key<D>) -> D>(
        &mut self,
        k: Key<D>,
        load_context: <D as OnLoad<'l>>::LoadContext,
        builder: B,
    ) -> Index<D>
    where
        D: OnLoad<'l>,
    {
        self.get_or_create(k, builder, |k, entry| {
            log::debug!("Request loading for [{:?}]", k);
            let load_token = LoadToken(Arc::downgrade(&entry.load_token), entry as *mut _, k.clone());
            entry.value.start_load(load_context, load_token);
        })
    }
}

/*
/// A context for load opration to handle notification.
pub struct LoadContext<'a, D: Data> {
    key: &'a Key<D>,
    load_response_sender: &'a UnboundedSender<(D::LoadResponse, CancellationToken<D>)>,
    load_token: &'a CancellationToken<D>,
}

impl<'a, D: Data> LoadContext<'a, D> {
    pub fn key(&self) -> Option<&D::Key> {
        match self.key {
            Key::Named(ref key) => Some(key),
            _ => None,
        }
    }
}

impl<'a, D: Data> fmt::Debug for LoadContext<'a, D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.key)
    }
}*/

/*pub trait DataUpdater<'a, D: Data> {
    fn update<'u>(
        &mut self,
        load_context: LoadContext<'u, D>,
        data: &mut D,
        load_response: D::LoadResponse,
    ) -> Option<D::LoadRequest>;
}

pub trait DataLoader<D: Data>: 'static + Send + Sync {
    fn load<'a>(
        &'a mut self,
        request: D::LoadRequest,
        load_token: CancellationToken<D>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<D::LoadResponse>>>>;
}

struct NoDataLoader;

impl<D: Data> DataLoader<D> for NoDataLoader {
    fn load<'a>(
        &'a mut self,
        _request: D::LoadRequest,
        _load_token: CancellationToken<D>,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<D::LoadResponse>>>> {
        Box::pin(futures::future::ready(None))
    }
}

pub struct StoreLoader<D: Data> {
    load_request_receiver: UnboundedReceiver<(D::LoadRequest, CancellationToken<D>)>,
    load_response_sender: UnboundedSender<(D::LoadResponse, CancellationToken<D>)>,
    data_loader: Box<dyn DataLoader<D>>,
}

unsafe impl<D: Data> Send for StoreLoader<D> {}

impl<D: Data> StoreLoader<D> {
    async fn handle_one(&mut self) -> bool {
        let (data, load_token) = match self.load_request_receiver.next().await {
            Some((data, load_token)) => (data, load_token),
            None => {
                log::info!("Loader requests failed {:?}", TypeId::of::<D>());
                return false;
            }
        };

        if load_token.is_canceled() {
            return true;
        }
        let output = match self.data_loader.load(data, load_token.clone()).await {
            Some(output) => output,
            None => return true,
        };
        if load_token.is_canceled() {
            return true;
        }

        match self.load_response_sender.send((output, load_token)).await {
            Ok(_) => true,
            Err(err) => {
                log::info!("Loader response failed {:?}: {:?}", TypeId::of::<D>(), err);
                false
            }
        }
    }

    async fn run(&mut self) {
        while self.handle_one().await {}
    }

    #[cfg(feature = "native")]
    pub fn start(mut self) {
        use tokio::{runtime::Handle, task};
        task::spawn_blocking(move || {
            Handle::current().block_on(task::LocalSet::new().run_until(async move {
                task::spawn_local(async move {
                    self.run().await;
                })
                .await
                .unwrap()
            }))
        });
    }

    #[cfg(feature = "wasm")]
    pub fn start(mut self) {
        use wasm_bindgen_futures::spawn_local;
        spawn_local(async move {
            self.run().await;
        });
    }
}
*/
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
        Store {
            shared: RwLock::new(SharedData {
                entries: HashMap::new(),
            }),
            exclusive: Mutex::new(ExclusiveData {
                arena: Arena::new(page_size),
                entries: HashMap::new(),
                unnamed_id: 0,
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
                    "Entry leak: [{:?}] is still referenced, shared",
                    k
                );
            }

            let shared_entries = &mut shared.entries;
            for (k, (_, ptr)) in shared_entries {
                let entry = unsafe { &**ptr };
                debug_assert!(
                    !entry.has_reference(),
                    "Entry leak: [{:?}] is still referenced, exclusive",
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

impl<'a, D: Data> ReadGuard<'a, D> {
    /// Try to get the index of a resource by the key.
    /// This operation may block if item is not found in the shared store and the transient container is
    /// in use. (ex. A new item is constructed.)
    pub fn try_get(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &self.exclusive;

        let k = Key::Named(k.clone());
        shared.get(&k).or_else(|| {
            let exclusive = exclusive.lock().unwrap();
            exclusive.get(&k)
        })
    }

    /// Add a new item to the store.
    /// This operation may block if the transient container is in use. (ex. A new item is constructed.)
    pub fn add(&mut self, data: D) -> Index<D>
    where
        D: FromKey,
    {
        let mut exclusive = self.exclusive.lock().unwrap();

        exclusive.unnamed_id += 1;
        let k = Key::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_add(k, move |k| match &k {
            Key::Unnamed(_) => data,
            _ => unreachable!(),
        })
    }

    /// Try to get an item or create it from the key if not found.
    /// This operation may block but it ensures, an item is created (and stored) exactly once.
    pub fn get_or_add(&mut self, k: &D::Key) -> Index<D>
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

    /// Add a new item to the store and trigger loading with the given context
    /// This operation may block if the transient container is in use. (ex. A new item is constructed.)
    pub fn load<'l>(&mut self, data: D, load_context: <D as OnLoad<'l>>::LoadContext) -> Index<D>
    where
        D: OnLoad<'l>,
    {
        let mut exclusive = self.exclusive.lock().unwrap();

        exclusive.unnamed_id += 1;
        let k = Key::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_load(k, load_context, move |k| match &k {
            Key::Unnamed(_) => data,
            _ => unreachable!(),
        })
    }

    /// Try to get an item or create it from the key and trigger loading if not found.
    /// This operation may block but it ensures, an item is created (and stored) exactly once.
    pub fn get_or_load<'l>(&mut self, k: &D::Key, load_context: <D as OnLoad<'l>>::LoadContext) -> Index<D>
    where
        D: FromKey + OnLoad<'l>,
    {
        let shared = &mut self.shared;
        let exclusive = &mut self.exclusive;

        let k = Key::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            let mut exclusive = exclusive.lock().unwrap();
            exclusive.get_or_load(k, load_context, |k| match &k {
                Key::Named(ref k) => <D as FromKey>::from_key(k),
                _ => unreachable!(),
            })
        })
    }

    pub fn at<'i: 'a>(&self, index: &'i Index<D>) -> &D {
        // To release/modify the indexed object from the container,
        // one have to get mutable reference to the store,
        // but that would contradict to the borrow checker.
        unsafe { &index.entry().value }
    }
}

/// Guarded update access to a store
pub struct WriteGuard<'a, D: Data> {
    shared: RwLockWriteGuard<'a, SharedData<D>>,
    locked_exclusive: MutexGuard<'a, ExclusiveData<D>>,
}

impl<'a, D: Data> WriteGuard<'a, D> {
    /// Try to get the index of a resource by the key.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn try_get(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &self.locked_exclusive;

        let k = Key::Named(k.clone());
        exclusive.get(&k).or_else(|| shared.get(&k))
    }

    /// Add a new item to the store.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn add(&mut self, data: D) -> Index<D>
    where
        D: FromKey,
    {
        let exclusive = &mut self.locked_exclusive;

        exclusive.unnamed_id += 1;
        let k = Key::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_add(k, move |k| match &k {
            Key::Unnamed(_) => data,
            _ => unreachable!(),
        })
    }

    /// Try to get an item or create it from the key if not found.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn get_or_add(&mut self, k: &D::Key) -> Index<D>
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

    /// Add a new item to the store and trigger loading with the given context
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn load<'l>(&mut self, data: D, load_context: <D as OnLoad<'l>>::LoadContext) -> Index<D>
    where
        D: OnLoad<'l>,
    {
        let exclusive = &mut self.locked_exclusive;

        exclusive.unnamed_id += 1;
        let k = Key::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_load(k, load_context, move |k| match &k {
            Key::Unnamed(_) => data,
            _ => unreachable!(),
        })
    }

    /// Try to get an item or create it from the key and trigger loading if not found.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn get_or_load<'l>(&mut self, k: &D::Key, load_context: <D as OnLoad<'l>>::LoadContext) -> Index<D>
    where
        D: FromKey + OnLoad<'l>,
    {
        let shared = &mut self.shared;
        let exclusive = &mut self.locked_exclusive;

        let k = Key::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            exclusive.get_or_load(k, load_context, |k| match &k {
                Key::Named(ref k) => <D as FromKey>::from_key(k),
                _ => unreachable!(),
            })
        })
    }

    /// Returns if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.locked_exclusive.entries.is_empty() && self.shared.entries.is_empty()
    }

    /// Move all new (pending) entries into the shared container
    pub fn finalize_requests(&mut self) {
        self.shared.entries.extend(&mut self.locked_exclusive.entries.drain());
    }

    pub fn update<'l>(
        &mut self,
        update_context: <D as OnLoad<'l>>::UpdateContext,
        load_token: LoadToken<D>,
        response: <D as Load>::LoadResponse,
    ) where
        D: OnLoad<'l>,
    {
        if load_token.is_canceled() {
            return;
        }

        #[cfg(debug_assertions)]
        {
            let stored = {
                let shared = &self.shared;
                let exclusive = &self.locked_exclusive;
                let k = &load_token.2;
                exclusive
                    .entries
                    .get(k)
                    .or_else(|| shared.entries.get(k))
                    .map(|(_, p)| *p)
                    .unwrap_or(ptr::null_mut())
            };
            debug_assert!(
                stored == load_token.1,
                "Internal error, entry was altered while token is still valid"
            );
        }

        let entry = unsafe { &mut *load_token.1 };
        entry.value.on_loaded(update_context, response);
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
/*
/// Type eraser trait to notify listeners.
trait Listeners {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn notify(&mut self);
}

/// Listener of a specific type.
struct TypedListeners<D: Load> {
    load_response_sender: UnboundedSender<(<D as Load>::LoadResponse, CancellationToken<D>)>,
    listeners: Vec<(<D as Load>::LoadResponse, CancellationToken<D>)>,
}

impl<D: Load> TypedListeners<D> {
    fn add(&mut self, request: <D as Load>::LoadResponse, cancellation_token: CancellationToken<D>) {
        if !cancellation_token.is_canceled() {
            self.listeners.push((request, cancellation_token));
        }
    }
}

impl<D: Load> Listeners for TypedListeners<D> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn notify(&mut self) {
        for (request, cancellation_token) in self.listeners.drain(..) {
            if cancellation_token.is_canceled() {
                continue;
            }

            log::debug!("Notify dependency completed: [{:?}]", cancellation_token.2);
            if let Err(err) = self.load_response_sender.unbounded_send((request, cancellation_token)) {
                log::error!("Failed to notify store: {:?}", err);
            }
        }
    }
}

/// Manage listener waiting for load completion.
pub struct LoadListeners {
    listeners: Mutex<HashMap<TypeId, Box<dyn Listeners>>>,
}

impl LoadListeners {
    pub fn new() -> LoadListeners {
        LoadListeners {
            listeners: Mutex::new(HashMap::new()),
        }
    }

    pub fn add<'a, D: Load>(&self /*, load_context: &LoadContext<'a, D>*/, request: <D as Load>::LoadResponse) {
        /* if load_context.cancellation_token.is_canceled() {
            return;
        }

        let mut listeners = self.listeners.lock().unwrap();

        let listener = listeners.entry(TypeId::of::<TypedListeners<D>>()).or_insert_with(|| {
            Box::new(TypedListeners {
                load_response_sender: load_context.load_response_sender.clone(),
                listeners: Vec::new(),
            })
        });

        let listener = Any::downcast_mut::<TypedListeners<D>>(listener.as_any_mut()).unwrap();
        log::debug!("Add dependency listener: [{:?}]", load_context.cancellation_token.2);
        listener.add(request, load_context.cancellation_token.clone());*/
        unimplemented!()
    }

    pub fn notify_all(&self) {
        let mut listeners = self.listeners.lock().unwrap();
        {
            for (_, mut listener) in listeners.drain() {
                listener.notify();
            }
        }
    }
}

impl Default for LoadListeners {
    fn default() -> LoadListeners {
        LoadListeners::new()
    }
}
*/
