use crate::resources::{
    NamedResourceRead, NamedResourceWrite, Resource, ResourceIndex, ResourceName, ResourceRead, ResourceWrite,
    Resources,
};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
};

/// Store resource names for each resource types.
#[derive(Default, Debug)]
pub struct ResourceClaims {
    claims_by_query: HashMap<TypeId, (Vec<ResourceIndex>, Vec<ResourceIndex>)>,
    all_immutable: HashSet<ResourceIndex>,
    all_mutable: HashSet<ResourceIndex>,
}

impl ResourceClaims {
    fn store_immutable(&mut self, idx: ResourceIndex) {
        assert!(self.all_mutable.get(&idx).is_none()); // claimed a resources both as mutable and immutable
        self.all_immutable.insert(idx);
    }

    fn store_mutable(&mut self, idx: ResourceIndex) {
        assert!(self.all_immutable.get(&idx).is_none()); // claimed a resources both as mutable and immutable
        assert!(self.all_mutable.get(&idx).is_none()); // claimed a resources multiple times for mutation
        self.all_mutable.insert(idx);
    }

    pub fn add_claim<RId, I1, I2>(&mut self, immutable: I1, mutable: I2)
    where
        RId: 'static,
        I1: IntoIterator<Item = ResourceIndex>,
        I2: IntoIterator<Item = ResourceIndex>,
    {
        let rty = TypeId::of::<RId>();
        assert!(self.claims_by_query.get(&rty).is_none());
        let immutable = immutable
            .into_iter()
            .inspect(|idx| self.store_immutable(idx.clone()))
            .collect();
        let mutable = mutable
            .into_iter()
            .inspect(|idx| self.store_mutable(idx.clone()))
            .collect();
        let r = self.claims_by_query.insert(rty, (immutable, mutable));
        assert!(r.is_none());
    }

    pub fn get_claims<RId: 'static>(&self) -> Option<&(Vec<ResourceIndex>, Vec<ResourceIndex>)> {
        let rty = TypeId::of::<RId>();
        self.claims_by_query.get(&rty)
    }

    pub fn is_claimed_immutable(&self, id: &ResourceIndex) -> bool {
        self.all_immutable.contains(&id)
    }

    pub fn is_claimed_mutable(&self, id: &ResourceIndex) -> bool {
        self.all_mutable.contains(&id)
    }
}

pub trait ResourceQuery {
    type Fetch: for<'a> FetchResource<'a>;
    type Claim: ?Sized;

    fn add_default_claim(resource_claims: &mut ResourceClaims);
    fn add_extra_claim(claim: &Self::Claim, resource_claims: &mut ResourceClaims);
}

pub trait FetchResource<'a> {
    type Item;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item;
}

/// Shared borrow of a resource
pub struct Res<'a, T: Resource>(pub(crate) ResourceRead<'a, T>);

impl<'a, T: Resource> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// Helper to have uique type_id for each ResourceQuery implementation
struct ResQuery<T>(PhantomData<T>);

impl<'a, T: Resource> ResourceQuery for Res<'a, T> {
    type Fetch = FetchResourceRead<T>;
    type Claim = ();

    fn add_default_claim(resource_claims: &mut ResourceClaims) {
        resource_claims.add_claim::<ResQuery<T>, _, _>(Some(ResourceIndex::of::<T>(None)), None);
    }

    fn add_extra_claim(_claim: &Self::Claim, _resource_claims: &mut ResourceClaims) {}
}

pub struct FetchResourceRead<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchResourceRead<T> {
    type Item = Res<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item {
        assert!(resource_claims.is_claimed_immutable(&ResourceIndex::of::<T>(None)));
        Res(resources.get::<T>().unwrap())
    }
}

/// Unique borrow of resource
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

/// Helper to have uique type_id for each ResourceQuery implementation
struct ResMutQuery<T>(PhantomData<T>);

impl<'a, T: Resource> ResourceQuery for ResMut<'a, T> {
    type Fetch = FetchResourceWrite<T>;
    type Claim = ();

    fn add_default_claim(resource_claims: &mut ResourceClaims) {
        resource_claims.add_claim::<ResMutQuery<T>, _, _>(None, Some(ResourceIndex::of::<T>(None)));
    }

    fn add_extra_claim(_claim: &Self::Claim, _resource_claims: &mut ResourceClaims) {}
}

pub struct FetchResourceWrite<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchResourceWrite<T> {
    type Item = ResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item {
        assert!(resource_claims.is_claimed_mutable(&ResourceIndex::of::<T>(None)));
        ResMut(resources.get_mut::<T>().unwrap())
    }
}

/// Shared borrow of multiple named resources.
pub struct NamedRes<'a, T: Resource>(pub(crate) NamedResourceRead<'a, T>);

impl<'a, T: Resource> NamedRes<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a, T: Resource> Index<usize> for NamedRes<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

/// Helper to have uique type_id for each ResourceQuery implementation
struct NamedResQuery<T>(PhantomData<T>);

impl<'a, T: Resource> ResourceQuery for NamedRes<'a, T> {
    type Fetch = FetchNamedResourceRead<T>;
    type Claim = [ResourceName];

    fn add_default_claim(_resource_claims: &mut ResourceClaims) {}

    fn add_extra_claim(claim: &Self::Claim, resource_claims: &mut ResourceClaims) {
        let immutable = claim.iter().map(|c| ResourceIndex::of::<T>(Some(c.clone()))).clone();
        resource_claims.add_claim::<NamedResQuery<T>, _, _>(immutable, None);
    }
}

pub struct FetchNamedResourceRead<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchNamedResourceRead<T> {
    type Item = NamedRes<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item {
        let claims = resource_claims.get_claims::<NamedResQuery<T>>().unwrap();
        let names = claims.0.iter().map(|x| x.name().unwrap().to_owned());
        NamedRes(resources.get_with_names::<T, _>(names).unwrap())
    }
}

/// Unique borrow of multiple named resources.
pub struct NamedResMut<'a, T: Resource>(pub(crate) NamedResourceWrite<'a, T>);

impl<'a, T: Resource> NamedResMut<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a, T: Resource> Index<usize> for NamedResMut<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

impl<'a, T: Resource> IndexMut<usize> for NamedResMut<'a, T> {
    fn index_mut(&mut self, idx: usize) -> &mut T {
        &mut self.0[idx]
    }
}

/// Helper to have uique type_id for each ResourceQuery implementation
struct NamedResMutQuery<T>(PhantomData<T>);

impl<'a, T: Resource> ResourceQuery for NamedResMut<'a, T> {
    type Fetch = FetchNamedResourceWrite<T>;
    type Claim = [ResourceName];

    fn add_default_claim(_resource_claims: &mut ResourceClaims) {}

    fn add_extra_claim(claim: &Self::Claim, resource_claims: &mut ResourceClaims) {
        let mutable = claim.iter().map(|c| ResourceIndex::of::<T>(Some(c.clone()))).clone();
        resource_claims.add_claim::<NamedResMutQuery<T>, _, _>(None, mutable);
    }
}

pub struct FetchNamedResourceWrite<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchNamedResourceWrite<T> {
    type Item = NamedResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item {
        let claims = resource_claims.get_claims::<NamedResMutQuery<T>>().unwrap();
        let names = claims.1.iter().map(|x| x.name().unwrap().to_owned());
        NamedResMut(resources.get_mut_with_names::<T, _>(names).unwrap())
    }
}
