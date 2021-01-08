use crate::{
    resources::{
        Resource, ResourceId, ResourceMultiRead, ResourceMultiWrite, ResourceRead, ResourceTag, ResourceWrite,
        Resources,
    },
    scheduler::ResourceClaim,
    ECSError,
};
use serde::Serialize;

use std::{
    any::TypeId,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
};

pub trait IntoResourceClaim {
    fn into_claim(&self) -> ResourceClaim;
}

pub trait ResourceQuery {
    type Fetch: for<'a> FetchResource<'a, Self::Claim>;
    type Claim: 'static + Send + Sync + Default + IntoResourceClaim;
}

pub trait FetchResource<'a, Claim> {
    type Item;

    fn fetch<'r: 'a>(resources: &'r Resources, claim: &'r Claim) -> Result<Self::Item, ECSError>;
}

/// Claim for Res<'_, T>
#[derive(Debug)]
pub struct ResClaim<T: Resource>(PhantomData<fn(T)>);

impl<T: Resource> Default for ResClaim<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource> IntoResourceClaim for ResClaim<T> {
    fn into_claim(&self) -> ResourceClaim {
        ResourceClaim::new(Some((TypeId::of::<T>(), ResourceId::Global)), None)
    }
}

/// Fetch for Res<'_, T>
pub struct ResFetch<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, ResClaim<T>> for ResFetch<T> {
    type Item = Res<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, _claim: &'r ResClaim<T>) -> Result<Self::Item, ECSError> {
        Ok(Res(resources.get::<T>()?))
    }
}

/// Shared borrow of a global resource by type
pub struct Res<'a, T: Resource>(pub ResourceRead<'a, T>);

impl<'a, T: Resource> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<'a, T: Resource> ResourceQuery for Res<'a, T> {
    type Claim = ResClaim<T>;
    type Fetch = ResFetch<T>;
}

/// Claim for ResMut<'_, T>
#[derive(Debug)]
pub struct ResMutClaim<T: Resource>(PhantomData<fn(T)>);

impl<T: Resource> Default for ResMutClaim<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource> IntoResourceClaim for ResMutClaim<T> {
    fn into_claim(&self) -> ResourceClaim {
        ResourceClaim::new(None, Some((TypeId::of::<T>(), ResourceId::Global)))
    }
}

/// Fetch for ResMut<'_, T>
pub struct ResMutFetch<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, ResMutClaim<T>> for ResMutFetch<T> {
    type Item = ResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, _claim: &'r ResMutClaim<T>) -> Result<Self::Item, ECSError> {
        Ok(ResMut(resources.get_mut::<T>()?))
    }
}

/// Unique borrow of a global resource by type
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
    type Fetch = ResMutFetch<T>;
}

/// Claims for MultiRes<'_, T>. A list of resource ids.
pub struct MultiResClaim<T: Resource>(Vec<ResourceId>, PhantomData<fn(T)>);

impl<T: Resource> Default for MultiResClaim<T> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T: Resource> MultiResClaim<T> {
    pub fn new(ids: Vec<ResourceId>) -> Self {
        Self(ids, PhantomData)
    }

    pub fn append_ids<'a, I: IntoIterator<Item = ResourceId>>(&mut self, iter: I) {
        self.0.extend(iter);
    }

    pub fn try_append_tags<I>(&mut self, iter: I) -> Result<(), ECSError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let ids = iter
            .into_iter()
            .map(|tag| ResourceId::from_tag(tag.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        self.append_ids(ids);
        Ok(())
    }

    pub fn try_append_objects<'a, I, K>(&mut self, iter: I) -> Result<(), ECSError>
    where
        I: IntoIterator,
        I::Item: AsRef<K>,
        K: Serialize,
    {
        let ids = iter
            .into_iter()
            .map(|obj| ResourceId::from_object(obj.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        self.append_ids(ids);
        Ok(())
    }
}

impl<T: Resource> IntoResourceClaim for MultiResClaim<T> {
    fn into_claim(&self) -> ResourceClaim {
        let immutable = self.0.iter().map(|c| (TypeId::of::<T>(), c.clone()));
        ResourceClaim::new(immutable, None)
    }
}

impl<T: Resource> fmt::Debug for MultiResClaim<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_tuple("MultiResClaim");
        self.0.iter().for_each(|x| {
            dbg.field(&x);
        });
        dbg.finish()
    }
}

/// Fetch for MultiRes<'_, T>.
pub struct MultiResFetch<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, MultiResClaim<T>> for MultiResFetch<T> {
    type Item = MultiRes<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, claims: &'r MultiResClaim<T>) -> Result<Self::Item, ECSError> {
        let resources = resources.get_with_ids::<T, _>(&claims.0)?;
        Ok(MultiRes(resources, claims))
    }
}

/// Shared borrow of multiple resources of the same type
pub struct MultiRes<'a, T: Resource>(ResourceMultiRead<'a, T>, &'a MultiResClaim<T>);

impl<'a, T: Resource> MultiRes<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn claim(&self) -> &MultiResClaim<T> {
        &self.1
    }

    pub fn position_by_tag(&self, tag: &ResourceTag) -> Option<usize> {
        (self.1).0.iter().position(|x| match x {
            ResourceId::Tag(t) => t == tag,
            _ => false,
        })
    }
}

impl<'a, T: Resource> Index<usize> for MultiRes<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

impl<'a, T: Resource> ResourceQuery for MultiRes<'a, T> {
    type Claim = MultiResClaim<T>;
    type Fetch = MultiResFetch<T>;
}

/// Claims for MultiResMut<'_, T>. A list of resource ids.
pub struct MultiResMutClaim<T: Resource>(Vec<ResourceId>, PhantomData<fn(T)>);

impl<T: Resource> Default for MultiResMutClaim<T> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T: Resource> MultiResMutClaim<T> {
    pub fn new(ids: Vec<ResourceId>) -> Self {
        Self(ids, PhantomData)
    }

    pub fn append_ids<'a, I: IntoIterator<Item = ResourceId>>(&mut self, iter: I) -> &mut Self {
        self.0.extend(iter);
        self
    }

    pub fn try_append_tags<I>(&mut self, iter: I) -> Result<(), ECSError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let ids = iter
            .into_iter()
            .map(|tag| ResourceId::from_tag(tag.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        self.append_ids(ids);
        Ok(())
    }

    pub fn try_append_objects<I, K>(&mut self, iter: I) -> Result<(), ECSError>
    where
        I: IntoIterator,
        I::Item: AsRef<K>,
        K: Serialize,
    {
        let ids = iter
            .into_iter()
            .map(|obj| ResourceId::from_object(obj.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        self.append_ids(ids);
        Ok(())
    }
}

impl<T: Resource> IntoResourceClaim for MultiResMutClaim<T> {
    fn into_claim(&self) -> ResourceClaim {
        let mutable = self.0.iter().map(|c| (TypeId::of::<T>(), c.clone()));
        ResourceClaim::new(None, mutable)
    }
}

impl<T: Resource> fmt::Debug for MultiResMutClaim<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_tuple("MultiResMutClaim");
        self.0.iter().for_each(|x| {
            dbg.field(&x);
        });
        dbg.finish()
    }
}

/// Fetch for MultiRes<'_, T>.
pub struct MultiResMutFetch<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, MultiResMutClaim<T>> for MultiResMutFetch<T> {
    type Item = MultiResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, claims: &'r MultiResMutClaim<T>) -> Result<Self::Item, ECSError> {
        let resources = resources.get_mut_with_ids::<T, _>(&claims.0)?;
        Ok(MultiResMut(resources, claims))
    }
}

/// Shared borrow of multiple resources of the same type
pub struct MultiResMut<'a, T: Resource>(ResourceMultiWrite<'a, T>, &'a MultiResMutClaim<T>);

impl<'a, T: Resource> MultiResMut<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn claim(&self) -> &MultiResMutClaim<T> {
        &self.1
    }

    pub fn position_by_tag(&self, tag: &ResourceTag) -> Option<usize> {
        (self.1).0.iter().position(|x| match x {
            ResourceId::Tag(t) => t == tag,
            _ => false,
        })
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

impl<'a, T: Resource> ResourceQuery for MultiResMut<'a, T> {
    type Claim = MultiResMutClaim<T>;
    type Fetch = MultiResMutFetch<T>;
}
