use crate::core::arena::Arena;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard, Weak};
use std::{fmt, ptr};

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

/// Trait to load data from
pub trait OnLoad<'l>: Data {
    type UpdateContext: 'l;
    type LoadContext : Loader;

    fn on_load_request(&self, load_context: &mut Self::LoadContext, load_token: LoadToken<Self>)
    where
        Self: Sized;

    fn on_load_response(
        &mut self,
        load_context: &mut Self::LoadContext,
        update_context: Self::UpdateContext,
        load_token: LoadToken<Self>,
        load_response: <Self::LoadContext as Loader>::LoadResponse,
    ) where
        Self: Sized;
}

pub trait Loader {
    type LoadRequest: 'static + Send;
    type LoadResponse: 'static + Send;
}

enum EntityKey<D: Data> {
    Named(D::Key),
    Unnamed(usize),
}

impl<D: Data> EntityKey<D> {
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

impl<D: Data> PartialEq for EntityKey<D> {
    fn eq(&self, other: &Self) -> bool {
        match (&self, &other) {
            (EntityKey::Named(ref k1), EntityKey::Named(ref k2)) => k1 == k2,
            (EntityKey::Unnamed(ref k1), EntityKey::Unnamed(ref k2)) => k1 == k2,
            _ => false,
        }
    }
}

impl<D: Data> Eq for EntityKey<D> {}

impl<D: Data> Hash for EntityKey<D> {
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

impl<D: Data> Clone for EntityKey<D> {
    fn clone(&self) -> Self {
        match &self {
            EntityKey::Named(ref k) => EntityKey::Named(k.clone()),
            EntityKey::Unnamed(ref k) => EntityKey::Unnamed(k.clone()),
        }
    }
}

impl<D: Data> fmt::Debug for EntityKey<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            EntityKey::Named(ref k) => f.debug_tuple("Named").field(&k).finish(),
            EntityKey::Unnamed(ref id) => f.debug_tuple("Unnamed").field(&id).finish(),
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
pub struct LoadToken<D: Data>(Weak<()>, *mut Entry<D>, EntityKey<D>);

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

impl<D: Data> fmt::Debug for LoadToken<D> {
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
struct SharedData<D: Data> {
    entries: HashMap<EntityKey<D>, (usize, *mut Entry<D>)>,
}

impl<D: Data> SharedData<D> {
    fn get(&self, k: &EntityKey<D>) -> Option<Index<D>> {
        self.entries.get(k).map(|(_, ptr)| unsafe { Index::from_ptr(*ptr) })
    }
}

/// Shared data with exclusive access always.
/// This is a transient area for the newly created resources.
struct ExclusiveData<D: Data, L> {
    arena: Arena<Entry<D>>,
    entries: HashMap<EntityKey<D>, (usize, *mut Entry<D>)>,
    unnamed_id: usize,
    load_context: L,
}

impl<D: Data, L> ExclusiveData<D, L> {
    fn get(&self, k: &EntityKey<D>) -> Option<Index<D>> {
        self.entries.get(k).map(|(_, ptr)| unsafe { Index::from_ptr(*ptr) })
    }

    /// Get or create a new item.
    fn get_or_create<B, PB>(&mut self, k: EntityKey<D>, build: B, post_build: PB) -> Index<D>
    where
        B: FnOnce(&EntityKey<D>) -> D,
        PB: FnOnce(&mut L, &mut D, LoadToken<D>),
    {
        let entries = &mut self.entries;
        let arena = &mut self.arena;
        let load_context = &mut self.load_context;

        let (_, entry_ptr) = entries.entry(k.clone()).or_insert_with(|| {
            let value = build(&k);
            let entry = Entry {
                ref_count: AtomicUsize::new(0),
                load_token: Arc::new(()),
                value,
            };
            let (id, mut entry) = arena.allocate(entry);
            let token = LoadToken(Arc::downgrade(&entry.load_token), entry as *mut _, k);
            post_build(load_context, &mut entry.value, token);
            (id, entry as *mut _)
        });

        unsafe { Index::from_ptr(*entry_ptr) }
    }

    pub fn update<'l>(
        &mut self,
        update_context: <D as OnLoad<'l>>::UpdateContext,
        load_token: LoadToken<D>,
        response: <L as Loader>::LoadResponse,
    ) where
        D: OnLoad<'l, LoadContext = L>,
        L: Loader,
    {
        /*if load_token.is_canceled() {
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
        entry
            .value
            .on_load_response(&mut self.locked_exclusive.load_context, update_context, load_token, response);*/
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
pub struct Store<D: Data, L = ()> {
    shared: RwLock<SharedData<D>>,
    exclusive: Mutex<ExclusiveData<D, L>>,
}

unsafe impl<D: Data, L> Send for Store<D, L> {}
unsafe impl<D: Data, L> Sync for Store<D, L> {}

impl<D: Data> Store<D, ()> {
    /// Create a new store without the loading pipeline.
    pub(crate) fn new(page_size: usize) -> Store<D, ()> {
        Store {
            shared: RwLock::new(SharedData {
                entries: HashMap::new(),
            }),
            exclusive: Mutex::new(ExclusiveData {
                arena: Arena::new(page_size),
                entries: HashMap::new(),
                unnamed_id: 0,
                load_context: (),
            }),
        }
    }
}

impl<D, L> Store<D, L>
where
    D: for<'l> OnLoad<'l, LoadContext = L>,
{
    /// Create a new store without the loading pipeline.
    pub(crate) fn new_with_load(page_size: usize, load_context: L) -> Store<D, L> {
        Store {
            shared: RwLock::new(SharedData {
                entries: HashMap::new(),
            }),
            exclusive: Mutex::new(ExclusiveData {
                arena: Arena::new(page_size),
                entries: HashMap::new(),
                unnamed_id: 0,
                load_context: load_context,
            }),
        }
    }
}

impl<D: Data, L> Store<D, L> {
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

impl<D: Data, L> Drop for Store<D, L> {
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
pub struct ReadGuard<'a, D: Data, L> {
    shared: RwLockReadGuard<'a, SharedData<D>>,
    exclusive: &'a Mutex<ExclusiveData<D, L>>,
}

impl<'a, D: Data, L> ReadGuard<'a, D, L> {
    /// Try to get the index of a resource by the key.
    /// This operation may block if item is not found in the shared store and the transient container is
    /// in use. (ex. A new item is constructed.)
    pub fn try_get(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &self.exclusive;

        let k = EntityKey::Named(k.clone());
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
        let k = EntityKey::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_create(k, move |k| data, move |_, _, _| {})
    }

    /// Try to get an item or create it from the key if not found.
    /// This operation may block but it ensures, an item is created (and stored) exactly once.
    pub fn get_or_add(&mut self, k: &D::Key) -> Index<D>
    where
        D: FromKey,
    {
        let shared = &mut self.shared;
        let exclusive = &mut self.exclusive;

        let k = EntityKey::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            let mut exclusive = exclusive.lock().unwrap();
            exclusive.get_or_create(k, move |k| <D as FromKey>::from_key(k.name()), move |_, _, _| {})
        })
    }

    /// Add a new item to the store and trigger loading with the given context
    /// This operation may block if the transient container is in use. (ex. A new item is constructed.)
    pub fn load(&mut self, data: D) -> Index<D>
    where
        D: for<'l> OnLoad<'l, LoadContext = L>,
    {
        let mut exclusive = self.exclusive.lock().unwrap();

        exclusive.unnamed_id += 1;
        let k = EntityKey::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_create(
            k,
            move |k| data,
            move |load_context, entity, token| entity.on_load_request(load_context, token),
        )
    }

    /// Try to get an item or create it from the key and trigger loading if not found.
    /// This operation may block but it ensures, an item is created (and stored) exactly once.
    pub fn get_or_load(&mut self, k: &D::Key) -> Index<D>
    where
        D: FromKey + for <'l> OnLoad<LoadContext = L>,
    {
        let shared = &mut self.shared;
        let exclusive = &mut self.exclusive;

        let k = EntityKey::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            let mut exclusive = exclusive.lock().unwrap();
            exclusive.get_or_create(
                k,
                move |k| <D as FromKey>::from_key(k.name()),
                move |load_context, entity, token| entity.on_load_request(load_context, token),
            )
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
pub struct WriteGuard<'a, D: Data, L = ()> {
    shared: RwLockWriteGuard<'a, SharedData<D>>,
    locked_exclusive: MutexGuard<'a, ExclusiveData<D, L>>,
}

impl<'a, D: Data, L> WriteGuard<'a, D, L> {
    /// Try to get the index of a resource by the key.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn try_get(&self, k: &D::Key) -> Option<Index<D>> {
        let shared = &self.shared;
        let exclusive = &self.locked_exclusive;

        let k = EntityKey::Named(k.clone());
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
        let k = EntityKey::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_create(k, move |k| data, move |_, _, _| {})
    }

    /// Try to get an item or create it from the key if not found.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn get_or_add(&mut self, k: &D::Key) -> Index<D>
    where
        D: FromKey,
    {
        let shared = &mut self.shared;
        let exclusive = &mut self.locked_exclusive;

        let k = EntityKey::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            exclusive.get_or_create(k, move |k| <D as FromKey>::from_key(k.name()), move |_, _, _| {})
        })
    }

    /// Add a new item to the store and trigger loading with the given context
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn load(&mut self, data: D) -> Index<D>
    where
        D: for <'l> OnLoad<LoadContext = L>,
    {
        let exclusive = &mut self.locked_exclusive;

        exclusive.unnamed_id += 1;
        let k = EntityKey::Unnamed(exclusive.unnamed_id);
        exclusive.get_or_create(
            k,
            move |k| data,
            move |load_context, entity, token| entity.on_load_request(load_context, token),
        )
    }

    /// Try to get an item or create it from the key and trigger loading if not found.
    /// This operation never blocks as WriteGueard has an exclusive access to the Store
    pub fn get_or_load<'l>(&mut self, k: &D::Key) -> Index<D>
    where
        D: FromKey + OnLoad<'l, LoadContext = L>,
    {
        let shared = &mut self.shared;
        let exclusive = &mut self.locked_exclusive;

        let k = EntityKey::Named(k.clone());
        shared.get(&k).unwrap_or_else(|| {
            exclusive.get_or_create(
                k,
                move |k| <D as FromKey>::from_key(k.name()),
                move |load_context, entity, token| entity.on_load_request(load_context, token),
            )
        })
    }

    /// Returns if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.locked_exclusive.entries.is_empty() && self.shared.entries.is_empty()
    }

    /// Move all new (pending) entries into the shared container
    pub fn finalize_requests(&mut self) {
        self.shared.entries.extend(&mut self.locked_exclusive.entries.drain());
        //self.locked_exclusive
    }

    fn drain_unused_filtered_impl<F: FnMut(&mut D) -> bool>(
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
