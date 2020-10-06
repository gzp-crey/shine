use crate::{
    core::ids::IdError,
    resources::{
        NamedResourceRead, NamedResourceWrite, Resource, ResourceClaim, ResourceIndex, ResourceName, ResourceRead,
        ResourceWrite, Resources,
    },
};
use std::{
    convert::TryFrom,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
    str::FromStr,
};

pub trait IntoResourceClaim: 'static + Send + Sync {
    fn into_claim(&self) -> ResourceClaim;
}

pub trait ResourceQuery {
    type Fetch: for<'a> FetchResource<'a, Self::Claim>;
    type Claim: Default + IntoResourceClaim;

    fn default_claims() -> ResourceClaim;
}

pub trait FetchResource<'a, Claim> {
    type Item;

    fn fetch<'r: 'a>(resources: &'r Resources, extra_claim: &'r Claim) -> Self::Item;
}

#[derive(Debug)]
pub struct ResClaim<T: Resource>(PhantomData<fn(T)>);

impl<T: Resource> Default for ResClaim<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource> IntoResourceClaim for ResClaim<T> {
    fn into_claim(&self) -> ResourceClaim {
        ResourceClaim::none()
    }
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
    type Claim = ResClaim<T>;
    type Fetch = FetchResourceRead<T>;

    fn default_claims() -> ResourceClaim {
        ResourceClaim::new(Some(ResourceIndex::new::<T>(None)), None)
    }
}

pub struct FetchResourceRead<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, ResClaim<T>> for FetchResourceRead<T> {
    type Item = Res<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, _extra_claim: &'r ResClaim<T>) -> Self::Item {
        Res(resources.get::<T>().unwrap())
    }
}

#[derive(Debug)]
pub struct ResMutClaim<T: Resource>(PhantomData<fn(T)>);

impl<T: Resource> Default for ResMutClaim<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource> IntoResourceClaim for ResMutClaim<T> {
    fn into_claim(&self) -> ResourceClaim {
        ResourceClaim::none()
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
    type Claim = ResMutClaim<T>;
    type Fetch = FetchResourceWrite<T>;

    fn default_claims() -> ResourceClaim {
        ResourceClaim::new(None, Some(ResourceIndex::new::<T>(None)))
    }
}

pub struct FetchResourceWrite<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, ResMutClaim<T>> for FetchResourceWrite<T> {
    type Item = ResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, _extra_claim: &'r ResMutClaim<T>) -> Self::Item {
        ResMut(resources.get_mut::<T>().unwrap())
    }
}

/// List of resource names for the shared borrower, [NamedRes]
pub struct NamedResClaim<T: Resource>(Vec<ResourceName>, PhantomData<fn(T)>);

impl<T: Resource> IntoResourceClaim for NamedResClaim<T> {
    fn into_claim(&self) -> ResourceClaim {
        let immutable = self.0.iter().map(|c| ResourceIndex::new::<T>(Some(c.clone())));
        ResourceClaim::new(immutable, None)
    }
}

impl<T: Resource> Default for NamedResClaim<T> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T: Resource> Deref for NamedResClaim<T> {
    type Target = Vec<ResourceName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Resource> DerefMut for NamedResClaim<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Resource> fmt::Debug for NamedResClaim<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_tuple("NamedResClaim");
        self.0.iter().for_each(|x| {
            dbg.field(&x.as_str());
        });
        dbg.finish()
    }
}

impl<'a, 'b, T: Resource> TryFrom<&'a [&'b str]> for NamedResClaim<T> {
    type Error = IdError;

    fn try_from(value: &'a [&'b str]) -> Result<Self, Self::Error> {
        let names = value
            .into_iter()
            .map(|name| ResourceName::from_str(name))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(names, PhantomData))
    }
}

/// Shared borrow of multiple named resources.
pub struct NamedRes<'a, T: Resource>(NamedResourceRead<'a, T>, &'a NamedResClaim<T>);

impl<'a, T: Resource> NamedRes<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn claim(&self) -> &NamedResClaim<T> {
        &self.1
    }

    pub fn position_by_name(&self, name: &ResourceName) -> Option<usize> {
        self.1.iter().position(|x| x == name)
    }
}

impl<'a, T: Resource> Index<usize> for NamedRes<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

impl<'a, T: Resource> ResourceQuery for NamedRes<'a, T> {
    type Claim = NamedResClaim<T>;
    type Fetch = FetchNamedResourceRead<T>;

    fn default_claims() -> ResourceClaim {
        ResourceClaim::none()
    }
}

pub struct FetchNamedResourceRead<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, NamedResClaim<T>> for FetchNamedResourceRead<T> {
    type Item = NamedRes<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, extra_claim: &'r NamedResClaim<T>) -> Self::Item {
        let resources = resources.get_with_names::<T, _>(extra_claim.iter()).unwrap();
        NamedRes(resources, extra_claim)
    }
}

/// List of resource names for the unique borrower, [NamedResMut]
pub struct NamedResMutClaim<T: Resource>(Vec<ResourceName>, PhantomData<fn(T)>);

impl<T: Resource> IntoResourceClaim for NamedResMutClaim<T> {
    fn into_claim(&self) -> ResourceClaim {
        let mutable = self.0.iter().map(|c| ResourceIndex::new::<T>(Some(c.clone())));
        ResourceClaim::new(None, mutable)
    }
}

impl<T: Resource> Default for NamedResMutClaim<T> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T: Resource> Deref for NamedResMutClaim<T> {
    type Target = Vec<ResourceName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Resource> DerefMut for NamedResMutClaim<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Resource> fmt::Debug for NamedResMutClaim<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_tuple("NamedResMutClaim");
        self.0.iter().for_each(|x| {
            dbg.field(&x.as_str());
        });
        dbg.finish()
    }
}

impl<'a, 'b, T: Resource> TryFrom<&'a [&'b str]> for NamedResMutClaim<T> {
    type Error = IdError;

    fn try_from(value: &'a [&'b str]) -> Result<Self, Self::Error> {
        let names = value
            .into_iter()
            .map(|name| ResourceName::from_str(name))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(names, PhantomData))
    }
}

/// Unique borrow of multiple named resources.
pub struct NamedResMut<'a, T: Resource>(NamedResourceWrite<'a, T>, &'a NamedResMutClaim<T>);

impl<'a, T: Resource> NamedResMut<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn claim(&self) -> &NamedResMutClaim<T> {
        &self.1
    }

    pub fn position_by_name(&self, name: &ResourceName) -> Option<usize> {
        self.1.iter().position(|x| x == name)
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
    type Claim = NamedResMutClaim<T>;

    fn default_claims() -> ResourceClaim {
        ResourceClaim::none()
    }
}

pub struct FetchNamedResourceWrite<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, NamedResMutClaim<T>> for FetchNamedResourceWrite<T> {
    type Item = NamedResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, extra_claim: &'r NamedResMutClaim<T>) -> Self::Item {
        let resources = resources.get_mut_with_names::<T, _>(extra_claim.iter()).unwrap();
        NamedResMut(resources, extra_claim)
    }
}
