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

#[derive(Debug, Clone, Copy)]
pub enum ResourceClaimScope {
    Default,
    Extra,
}

#[derive(Default, Debug)]
pub struct ResourceClaim {
    pub immutable: Vec<ResourceIndex>,
    pub mutable: Vec<ResourceIndex>,
}

impl ResourceClaim {
    pub fn new<I1, I2>(immutable: I1, mutable: I2) -> Self
    where
        I1: IntoIterator<Item = ResourceIndex>,
        I2: IntoIterator<Item = ResourceIndex>,
    {
        Self {
            immutable: immutable.into_iter().collect(),
            mutable: mutable.into_iter().collect(),
        }
    }
}

/// Store resource names for each resource types.
#[derive(Default, Debug)]
pub struct ResourceClaims {
    default_claims: HashMap<TypeId, ResourceClaim>,
    extra_claims: HashMap<TypeId, ResourceClaim>,
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

    fn claim_for(&self, scope: ResourceClaimScope) -> &HashMap<TypeId, ResourceClaim> {
        match scope {
            ResourceClaimScope::Default => &self.default_claims,
            ResourceClaimScope::Extra => &self.extra_claims,
        }
    }

    fn claim_mut_for(&mut self, scope: ResourceClaimScope) -> &mut HashMap<TypeId, ResourceClaim> {
        match scope {
            ResourceClaimScope::Default => &mut self.default_claims,
            ResourceClaimScope::Extra => &mut self.extra_claims,
        }
    }

    pub fn add_claim<RId: 'static>(&mut self, scope: ResourceClaimScope, claim: ResourceClaim) {
        let rty = TypeId::of::<RId>();
        claim.immutable.iter().for_each(|x| self.store_immutable(x.clone()));
        claim.mutable.iter().for_each(|x| self.store_mutable(x.clone()));
        let r = self.claim_mut_for(scope).insert(rty, claim);
        assert!(r.is_none());
    }

    pub fn get_claims<RId: 'static>(&self, scope: ResourceClaimScope) -> Option<&ResourceClaim> {
        let rty = TypeId::of::<RId>();
        self.claim_for(scope).get(&rty)
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
    type Claim;

    fn add_default_claim(resource_claims: &mut ResourceClaims);
    fn add_extra_claim(claim: Self::Claim, resource_claims: &mut ResourceClaims);
}

pub trait FetchResource<'a> {
    type Item;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item;
}

/// Shared borrow of a resource
pub struct Res<'a, T: Resource>(pub ResourceRead<'a, T>);

impl<'a, T: Resource> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<'a, T: Resource> ResourceQuery for Res<'a, T> {
    type Fetch = FetchResourceRead<T>;
    type Claim = ();

    fn add_default_claim(resource_claims: &mut ResourceClaims) {
        resource_claims.add_claim::<Self::Fetch>(
            ResourceClaimScope::Default,
            ResourceClaim::new(Some(ResourceIndex::new::<T>(None)), None),
        );
    }

    fn add_extra_claim(_claim: Self::Claim, _resource_claims: &mut ResourceClaims) {}
}

pub struct FetchResourceRead<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchResourceRead<T> {
    type Item = Res<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item {
        debug_assert!(resource_claims.is_claimed_immutable(&ResourceIndex::new::<T>(None)));
        Res(resources.get::<T>().unwrap())
    }
}

/// Unique borrow of resource
pub struct ResMut<'a, T: Resource>(pub ResourceWrite<'a, T>);

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

impl<'a, T: Resource> ResourceQuery for ResMut<'a, T> {
    type Fetch = FetchResourceWrite<T>;
    type Claim = ();

    fn add_default_claim(resource_claims: &mut ResourceClaims) {
        resource_claims.add_claim::<Self::Fetch>(
            ResourceClaimScope::Default,
            ResourceClaim::new(None, Some(ResourceIndex::new::<T>(None))),
        );
    }

    fn add_extra_claim(_claim: Self::Claim, _resource_claims: &mut ResourceClaims) {}
}

pub struct FetchResourceWrite<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchResourceWrite<T> {
    type Item = ResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item {
        debug_assert!(resource_claims.is_claimed_mutable(&ResourceIndex::new::<T>(None)));
        ResMut(resources.get_mut::<T>().unwrap())
    }
}

/// Shared borrow of multiple named resources.
pub struct NamedRes<'a, T: Resource>(pub NamedResourceRead<'a, T>);

impl<'a, T: Resource> NamedRes<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn position_by_name(&self, name: &ResourceName) -> Option<usize> {
        self.0.position_by_name(name)
    }
}

impl<'a, T: Resource> Index<usize> for NamedRes<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

impl<'a, T: Resource> ResourceQuery for NamedRes<'a, T> {
    type Fetch = FetchNamedResourceRead<T>;
    type Claim = Vec<ResourceName>;

    fn add_default_claim(_resource_claims: &mut ResourceClaims) {}

    fn add_extra_claim(claim: Self::Claim, resource_claims: &mut ResourceClaims) {
        let immutable = claim.into_iter().map(|c| ResourceIndex::new::<T>(Some(c)));
        resource_claims.add_claim::<Self::Fetch>(ResourceClaimScope::Extra, ResourceClaim::new(immutable, None));
    }
}

pub struct FetchNamedResourceRead<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchNamedResourceRead<T> {
    type Item = NamedRes<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item {
        let claims = resource_claims.get_claims::<Self>(ResourceClaimScope::Extra).unwrap();
        let names = claims.immutable.iter().map(|x| x.name().unwrap().to_owned());
        NamedRes(resources.get_with_names::<T, _>(names).unwrap())
    }
}

/// Unique borrow of multiple named resources.
pub struct NamedResMut<'a, T: Resource>(pub NamedResourceWrite<'a, T>);

impl<'a, T: Resource> NamedResMut<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn position_by_name(&self, name: &ResourceName) -> Option<usize> {
        self.0.position_by_name(name)
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

impl<'a, T: Resource> ResourceQuery for NamedResMut<'a, T> {
    type Fetch = FetchNamedResourceWrite<T>;
    type Claim = Vec<ResourceName>;

    fn add_default_claim(_resource_claims: &mut ResourceClaims) {}

    fn add_extra_claim(claim: Self::Claim, resource_claims: &mut ResourceClaims) {
        let mutable = claim.into_iter().map(|c| ResourceIndex::new::<T>(Some(c)));
        resource_claims.add_claim::<Self::Fetch>(ResourceClaimScope::Extra, ResourceClaim::new(None, mutable));
    }
}

pub struct FetchNamedResourceWrite<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a> for FetchNamedResourceWrite<T> {
    type Item = NamedResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, resource_claims: &'r ResourceClaims) -> Self::Item {
        let claims = resource_claims.get_claims::<Self>(ResourceClaimScope::Extra).unwrap();
        let names = claims.mutable.iter().map(|x| x.name().unwrap().to_owned());
        NamedResMut(resources.get_mut_with_names::<T, _>(names).unwrap())
    }
}
