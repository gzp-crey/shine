use crate::resources::{
    MultiResourceRead, MultiResourceWrite, Resource, ResourceIndex, ResourceName, ResourceRead, ResourceWrite,
    Resources,
};
use std::{
    any::{self, TypeId},
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
};

/// Shared borrow of an unnamed resource
pub struct Res<'a, T: Resource>(pub(crate) ResourceRead<'a, T>);

impl<'a, T: Resource> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// Unique borrow of a single unnamed resource
pub struct ResMut<'a, T: Resource>(pub(crate) ResourceWrite<'a, T>);

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
pub struct MultiRes<'a, T: Resource>(pub(crate) MultiResourceRead<'a, T>);

impl<'a, T: Resource> MultiRes<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a, T: Resource> Index<usize> for MultiRes<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

/// Unique borrow of multiple named resources.
pub struct MultiResMut<'a, T: Resource>(pub(crate) MultiResourceWrite<'a, T>);

impl<'a, T: Resource> MultiResMut<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

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
#[derive(Default, Debug)]
pub struct MultiResourceClaims {
    immutable: HashMap<TypeId, Vec<Option<ResourceName>>>,
    mutable: HashMap<TypeId, Vec<Option<ResourceName>>>,
}

/// Provides information about the resources a [System] reads and writes
#[derive(Default)]
pub struct ResourceAccess {
    multi_claims: MultiResourceClaims,
    all_immutable: HashSet<ResourceIndex>,
    all_mutable: HashSet<ResourceIndex>,
}

impl ResourceAccess {
    fn store_immutable(&mut self, idx: ResourceIndex) {
        assert!(self.all_mutable.get(&idx).is_none()); // claimed a resources both as mutable and immutable
        self.all_immutable.insert(idx);
    }

    pub fn add_immutable_claim<T: Resource>(&mut self) {
        self.store_immutable(ResourceIndex::of::<T>(None));
    }

    pub fn add_immutable_multi_claims<T: Resource>(&mut self, claims: &MultiResourceClaims) {
        let ty = TypeId::of::<T>();
        assert!(self.multi_claims.immutable.get(&ty).is_none()); // immutable multi resources claim already resolved for this type
        log::debug!(
            "Adding immutable claims to {} ({:?}) from {:?}",
            any::type_name::<T>(),
            ty,
            claims
        );
        if let Some(names) = claims.immutable.get(&ty) {
            let mut multi_entry = Vec::with_capacity(names.len());
            for name in names {
                let idx = ResourceIndex::of::<T>(name.clone());
                multi_entry.push(name.clone());
                self.store_immutable(idx);
            }
            let r = self.multi_claims.immutable.insert(ty, multi_entry);
            assert!(r.is_none());
        }
    }

    pub fn get_immutable_multi_claims(&self) -> &HashMap<TypeId, Vec<Option<ResourceName>>> {
        &self.multi_claims.immutable
    }

    pub fn get_immutable_multi_names<T: Resource>(&self) -> Option<&[Option<ResourceName>]> {
        let ty = TypeId::of::<T>();
        self.multi_claims.immutable.get(&ty).map(|v| &v[..])
    }

    fn store_mutable(&mut self, idx: ResourceIndex) {
        assert!(self.all_immutable.get(&idx).is_none()); // claimed a resources both as mutable and immutable
        assert!(self.all_mutable.get(&idx).is_none()); // claimed a resources multiple times for mutation
        self.all_mutable.insert(idx);
    }

    pub fn add_mutable_claim<T: Resource>(&mut self) {
        self.store_mutable(ResourceIndex::of::<T>(None));
    }

    pub fn add_mutable_multi_claims<T: Resource>(&mut self, claims: &MultiResourceClaims) {
        let ty = TypeId::of::<T>();
        log::debug!(
            "Adding mutable claims to {} ({:?}) from {:?}",
            any::type_name::<T>(),
            ty,
            claims
        );
        assert!(self.multi_claims.mutable.get(&ty).is_none()); // mutable multi resources claim already resolved for this type
        if let Some(names) = claims.mutable.get(&ty) {
            let mut multi_entry = Vec::with_capacity(names.len());
            for name in names {
                let idx = ResourceIndex::of::<T>(name.clone());
                multi_entry.push(name.clone());
                self.store_mutable(idx);
            }
            let r = self.multi_claims.mutable.insert(ty, multi_entry);
            assert!(r.is_none());
        }
    }

    pub fn get_mutable_multi_claims(&self) -> &HashMap<TypeId, Vec<Option<ResourceName>>> {
        &self.multi_claims.mutable
    }

    pub fn get_mutable_multi_names<T: Resource>(&self) -> Option<&[Option<ResourceName>]> {
        let ty = TypeId::of::<T>();
        self.multi_claims.mutable.get(&ty).map(|v| &v[..])
    }
}

pub trait FetchResource<'a> {
    type Item;

    /// Collect resource references
    fn access(multi_claims: &MultiResourceClaims, resource_access: &mut ResourceAccess);

    // Collect resource references
    fn fetch<'r: 'a>(resources: &'r Resources, resource_access: &'r ResourceAccess) -> Self::Item;
}

pub struct FetchResourceRead<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchResourceRead<T> {
    type Item = Res<'a, T>;

    fn access(_multi_claims: &MultiResourceClaims, resource_access: &mut ResourceAccess) {
        resource_access.add_immutable_claim::<T>();
    }

    fn fetch<'r: 'a>(resources: &'r Resources, _resource_access: &'r ResourceAccess) -> Self::Item {
        Res(resources.get::<T>(&None).unwrap())
    }
}

pub struct FetchResourceWrite<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchResourceWrite<T> {
    type Item = ResMut<'a, T>;

    fn access(_multi_resources: &MultiResourceClaims, resource_access: &mut ResourceAccess) {
        resource_access.add_mutable_claim::<T>();
    }

    fn fetch<'r: 'a>(resources: &'r Resources, _resource_access: &'r ResourceAccess) -> Self::Item {
        ResMut(resources.get_mut::<T>(&None).unwrap())
    }
}

pub struct FetchMultiResourceRead<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchMultiResourceRead<T> {
    type Item = MultiRes<'a, T>;

    fn access(multi_resources: &MultiResourceClaims, resource_access: &mut ResourceAccess) {
        resource_access.add_immutable_multi_claims::<T>(multi_resources);
    }

    fn fetch<'r: 'a>(resources: &'r Resources, resource_access: &'r ResourceAccess) -> Self::Item {
        log::debug!(
            "Fetch immutable resources for {} ({:?}) from {:?}",
            any::type_name::<T>(),
            TypeId::of::<T>(),
            resource_access.get_immutable_multi_claims()
        );
        let names = resource_access.get_immutable_multi_names::<T>().unwrap();
        MultiRes(resources.get_multi(names).unwrap())
    }
}

pub struct FetchMultiResourceWrite<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchMultiResourceWrite<T> {
    type Item = MultiResMut<'a, T>;

    fn access(multi_resources: &MultiResourceClaims, resource_access: &mut ResourceAccess) {
        resource_access.add_mutable_multi_claims::<T>(multi_resources);
    }

    fn fetch<'r: 'a>(resources: &'r Resources, resource_access: &'r ResourceAccess) -> Self::Item {
        log::debug!(
            "Fetch mutable resources for {} ({:?}) from {:?}",
            any::type_name::<T>(),
            TypeId::of::<T>(),
            resource_access.get_mutable_multi_claims()
        );
        let names = resource_access.get_mutable_multi_names::<T>().unwrap();
        MultiResMut(resources.get_multi_mut::<T>(names).unwrap())
    }
}

pub trait ResourceQuery {
    type Fetch: for<'a> FetchResource<'a>;

    fn add_claim(resource_claims: &mut MultiResourceClaims, claims: &[Option<ResourceName>]);
}

impl<'a, T: Resource> ResourceQuery for Res<'a, T> {
    type Fetch = FetchResourceRead<T>;

    fn add_claim(_resource_claims: &mut MultiResourceClaims, _claims: &[Option<ResourceName>]) {
        panic!("Use MultiRes to claim multiple named resources for read");
    }
}

impl<'a, T: Resource> ResourceQuery for ResMut<'a, T> {
    type Fetch = FetchResourceWrite<T>;

    fn add_claim(_resource_claims: &mut MultiResourceClaims, _claims: &[Option<ResourceName>]) {
        panic!("Use MultiResMut to claim multiple named resources for write");
    }
}

impl<'a, T: Resource> ResourceQuery for MultiRes<'a, T> {
    type Fetch = FetchMultiResourceRead<T>;

    fn add_claim(resource_claims: &mut MultiResourceClaims, claims: &[Option<ResourceName>]) {
        let ty = TypeId::of::<T>();
        log::debug!(
            "Claim immutable resources for {} ({:?}): {:?}",
            any::type_name::<T>(),
            ty,
            claims
        );
        resource_claims
            .immutable
            .entry(ty)
            .or_default()
            .extend(claims.iter().cloned());
    }
}

impl<'a, T: Resource> ResourceQuery for MultiResMut<'a, T> {
    type Fetch = FetchMultiResourceWrite<T>;

    fn add_claim(resource_claims: &mut MultiResourceClaims, claims: &[Option<ResourceName>]) {
        let ty = TypeId::of::<T>();
        log::debug!(
            "Claim mutable resources for {} ({:?}): {:?}",
            any::type_name::<T>(),
            ty,
            claims
        );
        resource_claims
            .mutable
            .entry(ty)
            .or_default()
            .extend(claims.iter().cloned());
    }
}
