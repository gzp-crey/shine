use crate::core::arena::Arena;
use std::{
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak,
    },
};

/// Trait for the data stored in a Store
pub trait Data: 'static {
    type Key: Clone + Send + Eq + Hash + fmt::Debug;
}

/// Trait to construct data from a key
pub trait FromKey: Data {
    fn from_key(key: &Self::Key) -> Self
    where
        Self: Sized;
}

/// Handle finalizing data during moving items from the transient into the shared storage
pub trait OnLoading<'b>: Data {
    type LoadingContext: 'b;
}

/// Handle data load
pub trait OnLoad: for<'l> OnLoading<'l> {
    type LoadRequest: 'static + Send;
    type LoadResponse: 'static + Send;
    type LoadHandler: 'static;

    fn on_load_request(&mut self, load_handler: &mut Self::LoadHandler, load_token: LoadToken<Self>)
    where
        Self: Sized;

    fn on_load_response<'l>(
        &mut self,
        load_handler: &mut Self::LoadHandler,
        loading_context: &mut <Self as OnLoading<'l>>::LoadingContext,
        load_token: LoadToken<Self>,
        load_response: Self::LoadResponse,
    ) where
        Self: Sized;
}

pub trait LoadHandler<D>
where
    D: OnLoad,
{
    fn next_response(&mut self) -> Option<(LoadToken<D>, D::LoadResponse)>;
}

pub struct NoLoad;

enum EntityKey<D>
where
    D: Data,
{
    Named(D::Key),
    Unnamed(usize),
}

impl<D> EntityKey<D>
where
    D: Data,
{
    fn is_named(&self) -> bool {
        match &self {
            EntityKey::Named(_) => true,
            EntityKey::Unnamed(_) => false,
        }
    }

    fn name(&self) -> &D::Key {
        match &self {
            EntityKey::Named(name) => name,
            _ => unreachable!(),
        }
    }
}

impl<D> PartialEq for EntityKey<D>
where
    D: Data,
{
    fn eq(&self, other: &Self) -> bool {
        match (&self, &other) {
            (EntityKey::Named(ref k1), EntityKey::Named(ref k2)) => k1 == k2,
            (EntityKey::Unnamed(ref k1), EntityKey::Unnamed(ref k2)) => k1 == k2,
            _ => false,
        }
    }
}

impl<D> Eq for EntityKey<D> where D: Data {}

impl<D> Hash for EntityKey<D>
where
    D: Data,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self {
            EntityKey::Named(ref k) => {
                "n".hash(state);
                k.hash(state);
            }
            EntityKey::Unnamed(ref k) => {
                "u".hash(state);
                k.hash(state);
            }
        }
    }
}

impl<D> Clone for EntityKey<D>
where
    D: Data,
{
    fn clone(&self) -> Self {
        match &self {
            EntityKey::Named(k) => EntityKey::Named(k.clone()),
            EntityKey::Unnamed(k) => EntityKey::Unnamed(*k),
        }
    }
}

impl<D> fmt::Debug for EntityKey<D>
where
    D: Data,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            EntityKey::Named(ref k) => f.debug_tuple("Named").field(&k).finish(),
            EntityKey::Unnamed(ref id) => f.debug_tuple("Unnamed").field(&id).finish(),
        }
    }
}

/// Reference counted index to access stored items in O(1).
pub struct Index<D: Data>(*const Entry<D>);

unsafe impl<D> Send for Index<D> where D: Data {}
unsafe impl<D> Sync for Index<D> where D: Data {}

impl<D> Index<D>
where
    D: Data,
{
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

impl<D> PartialEq for Index<D>
where
    D: Data,
{
    fn eq(&self, e: &Self) -> bool {
        self.0 == e.0
    }
}

impl<D> Clone for Index<D>
where
    D: Data,
{
    fn clone(&self) -> Index<D> {
        let entry = unsafe { self.entry() };
        entry.ref_count.fetch_add(1, Ordering::Relaxed);
        Index(self.0)
    }
}

impl<D> Drop for Index<D>
where
    D: Data,
{
    fn drop(&mut self) {
        let entry = unsafe { self.entry() };
        entry.ref_count.fetch_sub(1, Ordering::Relaxed);
    }
}

/// An entry in the store.
struct Entry<D>
where
    D: Data,
{
    ref_count: AtomicUsize,
    load_token: Arc<()>,
    value: D,
}

impl<D> Entry<D>
where
    D: Data,
{
    fn has_reference(&self) -> bool {
        self.ref_count.load(Ordering::Relaxed) > 0
    }
}

pub struct LoadCanceled;

/// A token to test the cancelation of loading operations.
pub struct LoadToken<D: Data>(Weak<()>, *mut Entry<D>, EntityKey<D>);

unsafe impl<D> Send for LoadToken<D> where D: Data {}
unsafe impl<D> Sync for LoadToken<D> where D: Data {}

impl<D> LoadToken<D>
where
    D: Data,
{
    pub fn is_canceled(&self) -> bool {
        self.0.upgrade().is_none()
    }

    pub fn ok(&self) -> Result<(), LoadCanceled> {
        if self.is_canceled() {
            Err(LoadCanceled)
        } else {
            Ok(())
        }
    }
}

impl<D> Clone for LoadToken<D>
where
    D: Data,
{
    fn clone(&self) -> Self {
        LoadToken(self.0.clone(), self.1, self.2.clone())
    }
}

impl<D> fmt::Debug for LoadToken<D>
where
    D: Data,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_canceled() {
            write!(f, "!{:?}", self.2)
        } else {
            write!(f, "{:?}", self.2)
        }
    }
}

/// Shared data with multiple reador or single writer access.
/// This is the main data storage tho ensure no resources are altered during use.
struct SharedData<D>
where
    D: Data,
{
    entries: HashMap<EntityKey<D>, (usize, *mut Entry<D>)>,
}

impl<D> SharedData<D>
where
    D: Data,
{
    fn get(&self, k: &EntityKey<D>) -> Option<Index<D>> {
        self.entries.get(k).map(|(_, ptr)| unsafe { Index::from_ptr(*ptr) })
    }
}

/// Shared data with exclusive access always.
/// This is a transient area for the newly created resources.
struct ExclusiveData<D>
where
    D: Data,
{
    arena: Arena<Entry<D>>,
    entries: HashMap<EntityKey<D>, (usize, *mut Entry<D>)>,
    unnamed_id: usize,
}

impl<D> ExclusiveData<D>
where
    D: Data,
{
    fn get(&self, k: &EntityKey<D>) -> Option<Index<D>> {
        self.entries.get(k).map(|(_, ptr)| unsafe { Index::from_ptr(*ptr) })
    }

    /// Get or create a new item.
    fn get_or_create<B, PB>(&mut self, k: EntityKey<D>, build: B, post_build: PB) -> Index<D>
    where
        B: FnOnce(&EntityKey<D>) -> D,
        PB: FnOnce(&mut D, LoadToken<D>),
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
            let (id, entry) = arena.allocate(entry);
            let token = LoadToken(Arc::downgrade(&entry.load_token), entry as *mut _, k);
            log::debug!("[{:?}] Entry created", token);
            post_build(&mut entry.value, token);
            (id, entry as *mut _)
        });

        unsafe { Index::from_ptr(*entry_ptr) }
    }
}

/// Thread safe resource store.
/// While the store is locked for reading, no resource can be modified, but new one can be created:
/// - 1st the shared data is searched for an existing item (non-blocking)
/// - 2nd the mutex guarded shared data is used to find or create the resource.
///   (blocking for the time of creation and enqueing)
/// While the store is locked for write, resources can be modified and released:
/// - 1st entries are modifed on by the loading policy
/// - 2nd the items from the mutex guarded data are moved into the shared store.
/// - 3nd on requests entries are dropped if there are no external references left. Dispite of
///   having a reference count of zero for the sotred items, they are not reclaimed without an explicit
///   request.
pub struct Store<D, L = NoLoad>
where
    D: Data,
{
    shared: RwLock<SharedData<D>>,
    exclusive: Mutex<(ExclusiveData<D>, L)>,
}

unsafe impl<D, L> Send for Store<D, L> where D: Data {}
unsafe impl<D, L> Sync for Store<D, L> where D: Data {}

impl<D, L> Store<D, L>
where
    D: OnLoad<LoadHandler = L>,
    L: 'static + LoadHandler<D>,
{
    /// Create a new store without the loading pipeline.
    pub(crate) fn new_with_load(page_size: usize, load_handler: L) -> Store<D, L> {
        Store {
            shared: RwLock::new(SharedData {
                entries: HashMap::default(),
            }),
            exclusive: Mutex::new((
                ExclusiveData {
                    arena: Arena::new(page_size),
                    entries: HashMap::default(),
                    unnamed_id: 0,
                },
                load_handler,
            )),
        }
    }

    /// Bake and move completed (transient) entries into the shared container
    pub fn load_and_finalize_requests<'l>(&mut self, mut loading_context: <D as OnLoading<'l>>::LoadingContext) {
        let shared = &mut *self.shared.try_write().unwrap();
        let exclusive = &mut *self.exclusive.lock().unwrap();
        let (exclusive, load_handler) = exclusive;

        while let Some((load_token, load_response)) = load_handler.next_response() {
            log::trace!("[{:?}] Receive loading response", load_token);

            if load_token.is_canceled() {
                log::trace!("[{:?}] Resource cancaled", load_token);
                continue;
            }

            #[cfg(debug_assert)]
            {
                let stored = {
                    let shared = &self.shared;
                    let exclusive = &self.exclusive;
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
            log::trace!("[{:?}] On load response", load_token);
            entry
                .value
                .on_load_response(load_handler, &mut loading_context, load_token, load_response);
        }

        shared.entries.extend(exclusive.entries.drain());
    }
}

impl<D> Store<D, NoLoad>
where
    D: Data,
{
    /// Create a new store without the loading pipeline.
    pub(crate) fn new(page_size: usize) -> Store<D, NoLoad> {
        Store {
            shared: RwLock::new(SharedData {
                entries: HashMap::default(),
            }),
            exclusive: Mutex::new((
                ExclusiveData {
                    arena: Arena::new(page_size),
                    entries: HashMap::default(),
                    unnamed_id: 0,
                },
                NoLoad,
            )),
        }
    }

    /// Move all new (transient) entries into the shared container
    pub fn finalize_requests(&mut self) {
        let shared = &mut *self.shared.try_write().unwrap();
        let exclusive = &mut *self.exclusive.lock().unwrap();
        let (exclusive, _) = exclusive;

        shared.entries.extend(exclusive.entries.drain());
    }
}

impl<D, L> Store<D, L>
where
    D: Data,
{
    fn drain_unused_if_impl<F: FnMut(&mut D) -> bool>(
        arena: &mut Arena<Entry<D>>,
        entries: &mut HashMap<EntityKey<D>, (usize, *mut Entry<D>)>,
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
    pub fn drain_unused_if<F: FnMut(&mut D) -> bool>(&mut self, mut filter: F) {
        let shared = &mut *self.shared.try_write().unwrap();
        let exclusive = &mut *self.exclusive.lock().unwrap();
        let (exclusive, _) = exclusive;
        Self::drain_unused_if_impl(&mut exclusive.arena, &mut shared.entries, &mut filter);
        Self::drain_unused_if_impl(&mut exclusive.arena, &mut exclusive.entries, &mut filter);
    }

    /// Drain all unreferenced items.
    pub fn drain_unused(&mut self) {
        self.drain_unused_if(|_| true)
    }

    /// Returns if the store is empty.
    pub fn is_empty(&mut self) -> bool {
        let shared = &*self.shared.try_read().unwrap();
        let exclusive = &*self.exclusive.lock().unwrap();
        let (exclusive, _) = exclusive;

        exclusive.entries.is_empty() && shared.entries.is_empty()
    }

    /// Aquire read lock.
    pub fn try_read(&self) -> Option<ReadGuard<'_, D, L>> {
        let shared = self.shared.try_read().ok()?;
        Some(ReadGuard {
            shared,
            exclusive: &self.exclusive,
        })
    }

    /// Aquire read lock. In case of failure the function panics.
    pub fn read(&self) -> ReadGuard<'_, D, L> {
        self.try_read().unwrap()
    }

    /// Try to aquire write lock.
    pub fn try_write(&self) -> Option<WriteGuard<'_, D, L>> {
        let shared = self.shared.try_write().ok()?;
        let locked_exclusive = self.exclusive.lock().ok()?;
        Some(WriteGuard {
            shared,
            locked_exclusive,
        })
    }

    /// Aquire write lock. In case of failure the function panics.
    pub fn write(&mut self) -> WriteGuard<'_, D, L> {
        self.try_write().unwrap()
    }
}

impl<D, L> Drop for Store<D, L>
where
    D: Data,
{
    fn drop(&mut self) {
        let shared = &mut *(self.shared.try_write().unwrap());
        let exclusive = &mut *(self.exclusive.lock().unwrap());
        let (exclusive, _) = exclusive;
        let arena = &mut exclusive.arena;

        {
            let exclusive_entries = &mut exclusive.entries;
            for (k, (_, ptr)) in exclusive_entries {
                let entry = unsafe { &**ptr };
                if !entry.has_reference() {
                    log::warn!("Entry leak: {:?} is still referenced, shared", k);
                }
            }

            let shared_entries = &mut shared.entries;
            for (k, (_, ptr)) in shared_entries {
                let entry = unsafe { &**ptr };
                if !entry.has_reference() {
                    log::warn!("Entry leak: {:?} is still referenced, exclusive", k);
                }
            }
        }

        if !arena.is_empty() {
            log::warn!("Leaking entries");
        }
    }
}

/// Guarded read access to a store
pub struct ReadGuard<'g, D, L>
where
    D: Data,
{
    shared: RwLockReadGuard<'g, SharedData<D>>,
    exclusive: &'g Mutex<(ExclusiveData<D>, L)>,
}

impl<'g, D, L> ReadGuard<'g, D, L>
where
    D: Data,
{
    /// Try to get the index of a resource by the key.
    /// This operation may block if item is not found in the shared store and the transient container is
    /// in use. (ex. A new item is constructed.)
    pub fn try_get(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &self.exclusive;

        let k = EntityKey::Named(k.clone());
        shared.get(&k).or_else(|| {
            let exclusive = &mut *exclusive.lock().unwrap();
            let (exclusive, _) = exclusive;
            exclusive.get(&k)
        })
    }

    pub fn at<'s, 'i: 's>(&'s self, index: &'i Index<D>) -> &'s D {
        // To release/modify the indexed object from the container,
        // one have to get mutable reference to the store,
        // but that would contradict to the borrow checker.
        unsafe { &index.entry().value }
    }
}

impl<'g, D> ReadGuard<'g, D, NoLoad>
where
    D: FromKey,
{
    /// Try to get an item or create it from the key if not found.
    /// This operation may block but it ensures, an item is created (and stored) exactly once.
    pub fn get_or_add(&self, k: &D::Key) -> Index<D> {
        let shared = &self.shared;
        let exclusive = &self.exclusive;

        let k = EntityKey::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            let exclusive = &mut *exclusive.lock().unwrap();
            let (exclusive, _) = exclusive;
            exclusive.get_or_create(k, move |k| <D as FromKey>::from_key(k.name()), move |_, _| {})
        })
    }
}

impl<'g, D, L> ReadGuard<'g, D, L>
where
    D: FromKey + OnLoad<LoadHandler = L>,
    L: 'static,
{
    /// Try to get an item or create it from the key and trigger loading if not found.
    /// This operation may block but it ensures, an item is created (and stored) exactly once.
    pub fn get_or_load(&self, k: &D::Key) -> Index<D> {
        let shared = &self.shared;
        let exclusive = &self.exclusive;

        let k = EntityKey::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            let exclusive = &mut *exclusive.lock().unwrap();
            let (exclusive, load_handler) = exclusive;
            exclusive.get_or_create(
                k,
                move |k| <D as FromKey>::from_key(k.name()),
                move |entity, token| entity.on_load_request(load_handler, token),
            )
        })
    }
}

/// Guarded mutable access to a store
pub struct WriteGuard<'g, D, L>
where
    D: Data,
{
    shared: RwLockWriteGuard<'g, SharedData<D>>,
    locked_exclusive: MutexGuard<'g, (ExclusiveData<D>, L)>,
}

impl<'g, D, L> WriteGuard<'g, D, L>
where
    D: Data,
{
    /// Try to get the index of a resource by the key.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn try_get(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &*self.locked_exclusive;
        let (exclusive, _) = exclusive;

        let k = EntityKey::Named(k.clone());
        exclusive.get(&k).or_else(|| shared.get(&k))
    }

    pub fn at<'s, 'i: 's>(&'s mut self, index: &'i Index<D>) -> &'s D {
        // To release/modify the indexed object from the container,
        // one have to get mutable reference to the store,
        // but that would contradict to the borrow checker.
        unsafe { &index.entry().value }
    }

    pub fn at_mut<'s, 'i: 's>(&'s mut self, index: &'i Index<D>) -> &'s mut D {
        // To release/modify the indexed object from the container,
        // one have to get mutable reference to the store,
        // but that would contradict to the borrow checker.
        unsafe { &mut index.entry_mut().value }
    }
}

impl<'g, D> WriteGuard<'g, D, NoLoad>
where
    D: Data,
{
    /// Add a new item to the store with an auto-assigned key.
    /// As item has an auto-key, the object can be accessed only through index. If index is dropped the
    /// item cannot be retreived from the store any more.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn add(&mut self, data: D) -> Index<D>
    where
        D: FromKey,
    {
        let exclusive = &mut *self.locked_exclusive;
        let (exclusive, _) = exclusive;

        exclusive.unnamed_id += 1;
        let k = EntityKey::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_create(k, move |_| data, move |_, _| {})
    }

    /// Try to get an item or create it from the key if not found.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn get_or_add(&mut self, k: &D::Key) -> Index<D>
    where
        D: FromKey,
    {
        let shared = &mut self.shared;
        let exclusive = &mut *self.locked_exclusive;
        let (exclusive, _) = exclusive;

        let k = EntityKey::Named(k.clone());
        shared
            .get(&k)
            .unwrap_or_else(|| exclusive.get_or_create(k, move |k| <D as FromKey>::from_key(k.name()), move |_, _| {}))
    }
}

impl<'g, D, L> WriteGuard<'g, D, L>
where
    D: FromKey + OnLoad<LoadHandler = L>,
    L: 'static,
{
    /// Add a new item to the store with an auto-assigned key and trigger the loading.
    /// As item has an auto-key, the object can be accessed only through index. If index is dropped the
    /// item cannot be retreived from the store any more.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store.
    pub fn load(&mut self, data: D) -> Index<D>
    where
        D: OnLoad<LoadHandler = L>,
        L: 'static,
    {
        let exclusive = &mut *self.locked_exclusive;
        let (exclusive, load_handler) = exclusive;

        exclusive.unnamed_id += 1;
        let k = EntityKey::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_create(
            k,
            move |_| data,
            move |entity, token| entity.on_load_request(load_handler, token),
        )
    }

    /// Try to get an item or create it from the key and trigger loading if not found.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn get_or_load(&mut self, k: &D::Key) -> Index<D>
    where
        D: FromKey + OnLoad<LoadHandler = L>,
        L: 'static,
    {
        let shared = &mut self.shared;
        let exclusive = &mut *self.locked_exclusive;
        let (exclusive, load_handler) = exclusive;

        let k = EntityKey::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            exclusive.get_or_create(
                k,
                move |k| <D as FromKey>::from_key(k.name()),
                move |entity, token| entity.on_load_request(load_handler, token),
            )
        })
    }
}
