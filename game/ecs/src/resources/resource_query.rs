use crate::resources::{
    FetchMultiResourceRead, FetchMultiResourceWrite, FetchResourceRead, FetchResourceWrite, Resource, ResourceIndex,
};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    ops::{Deref, DerefMut, Index, IndexMut},
};

/// Shared borrow of an unnamed resource
pub struct Res<'a, T: Resource>(pub(crate) FetchResourceRead<'a, T>);

impl<'a, T: Resource> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// Unique borrow of a single unnamed resource
pub struct ResMut<'a, T: Resource>(pub(crate) FetchResourceWrite<'a, T>);

impl<'a, T: Resource> Deref for ResMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<'a, T: Resource> DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// Shared borrow of multiple named resources.
pub struct MultiRes<'a, T: Resource>(pub(crate) FetchMultiResourceRead<'a, T>);

impl<'a, T: Resource> Index<usize> for MultiRes<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

/// Unique borrow of multiple named resources.
pub struct MultiResMut<'a, T: Resource>(pub(crate) FetchMultiResourceWrite<'a, T>);

impl<'a, T: Resource> Index<usize> for MultiResMut<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

impl<'a, T: Resource> IndexMut<usize> for MultiResMut<'a, T> {
    fn index_mut(&mut self, idx: usize) -> &mut T {
        &mut self.0[idx]
    }
}

/// Store resource names for each resource types.
pub struct MultiResourceClaims {
    pub immutable: HashMap<TypeId, Vec<Option<String>>>,
    pub mutable: HashMap<TypeId, Vec<Option<String>>>,
}

/// Provides information about the resources a [System] reads and writes
pub struct ResourceAccess {
    multi_immutable: HashMap<TypeId, Vec<ResourceIndex>>,
    multi_mutable: HashMap<TypeId, Vec<ResourceIndex>>,

    all_immutable: HashSet<ResourceIndex>,
    all_mutable: HashSet<ResourceIndex>,
}

impl ResourceAccess {
    pub fn new() -> Self {
        Self {
            multi_immutable: HashMap::default(),
            multi_mutable: HashMap::default(),
            all_immutable: HashSet::default(),
            all_mutable: HashSet::default(),
        }
    }

    fn store_immutable(&mut self, idx: ResourceIndex) {
        assert!(self.all_mutable.get(&idx).is_none()); // claimed a resources both as mutable and immutable
        self.all_immutable.insert(idx);
    }

    pub fn add_immutable<T: Resource>(&mut self) {
        self.store_immutable(ResourceIndex::of::<T>(None));
    }

    pub fn add_immutable_multi<T: Resource>(&mut self, claims: &MultiResourceClaims) {
        let ty = TypeId::of::<T>();
        assert!(self.multi_immutable.get(&ty).is_none()); // immutable multi resources claim already resolved for this type
        if let Some(names) = claims.immutable.get(&ty) {
            let mut multi_entry = Vec::with_capacity(names.len());
            for name in names {
                let idx = ResourceIndex::of::<T>(name.clone());
                multi_entry.push(idx.clone());
                self.store_immutable(idx);
            }
            self.multi_immutable.insert(ty, multi_entry);
        }
    }

    fn store_mutable(&mut self, idx: ResourceIndex) {
        assert!(self.all_immutable.get(&idx).is_none()); // claimed a resources both as mutable and immutable
        assert!(self.all_mutable.get(&idx).is_none()); // claimed a resources multiple times for mutation
        self.all_mutable.insert(idx);
    }

    pub fn add_mutable<T: Resource>(&mut self) {
        self.store_mutable(ResourceIndex::of::<T>(None));
    }

    pub fn add_mutable_multi<T: Resource>(&mut self, claims: &MultiResourceClaims) {
        let ty = TypeId::of::<T>();
        assert!(self.multi_mutable.get(&ty).is_none()); // mutable multi resources claim already resolved for this type
        if let Some(names) = claims.mutable.get(&ty) {
            let mut multi_entry = Vec::with_capacity(names.len());
            for name in names {
                let idx = ResourceIndex::of::<T>(name.clone());
                multi_entry.push(idx.clone());
                self.store_mutable(idx);
            }
            self.multi_mutable.insert(ty, multi_entry);
        }
    }
}

pub trait FetchResource {
    type Item;

    /// Collect the claimed resource indices
    fn access(multi_claims: &MultiResourceClaims, resoucres: &mut ResourceAccess);
}

impl<'a, T: Resource> FetchResource for FetchResourceRead<'a, T> {
    type Item = Res<'a, T>;

    fn access(_multi_claims: &MultiResourceClaims, resources: &mut ResourceAccess) {
        resources.add_immutable::<T>();
    }
}

impl<'a, T: Resource> FetchResource for FetchResourceWrite<'a, T> {
    type Item = ResMut<'a, T>;

    fn access(_multi_resources: &MultiResourceClaims, resources: &mut ResourceAccess) {
        resources.add_mutable::<T>();
    }
}

impl<'a, T: Resource> FetchResource for FetchMultiResourceRead<'a, T> {
    type Item = MultiRes<'a, T>;

    fn access(multi_resources: &MultiResourceClaims, resources: &mut ResourceAccess) {
        resources.add_immutable_multi::<T>(multi_resources);
    }
}

impl<'a, T: Resource> FetchResource for FetchMultiResourceWrite<'a, T> {
    type Item = MultiResMut<'a, T>;

    fn access(multi_resources: &MultiResourceClaims, resources: &mut ResourceAccess) {
        resources.add_mutable_multi::<T>(multi_resources);
    }
}

pub trait ResourceQuery {
    type Fetch: FetchResource;
}

impl<'a, T: Resource> ResourceQuery for Res<'a, T> {
    type Fetch = FetchResourceRead<'a, T>;
}

impl<'a, T: Resource> ResourceQuery for ResMut<'a, T> {
    type Fetch = FetchResourceWrite<'a, T>;
}

impl<'a, T: Resource> ResourceQuery for MultiRes<'a, T> {
    type Fetch = FetchMultiResourceRead<'a, T>;
}

impl<'a, T: Resource> ResourceQuery for MultiResMut<'a, T> {
    type Fetch = FetchMultiResourceWrite<'a, T>;
}
