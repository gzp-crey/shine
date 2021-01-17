use crate::{
    resources::{
        Resource, ResourceId, ResourceMultiRead, ResourceMultiWrite, ResourceRead, ResourceTag, ResourceWrite,
        Resources,
    },
    ECSError,
};
use serde::Serialize;

use std::{
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
    slice::Iter,
};

pub trait ResourceQuery: 'static + Default + Send + Sync + Sized {
    type Fetch: for<'a> FetchResource<'a, Self>;
}

pub trait ResourceAccess {
    type Fetch: for<'a> FetchResource<'a, Self::Query>;
    type Query: ResourceQuery;
}

pub trait FetchResource<'a, Query> {
    type Item;

    /// Fetch resource by the query. If supported the query is updated with some hints for faster
    /// lookup on repetative use.
    fn fetch<'r: 'a>(resources: &'r Resources, claim: &'r mut Query) -> Result<Self::Item, ECSError>;
}

impl Resources {
    pub fn claim<'r, Q: ResourceQuery>(
        &'r self,
        claim: &'r mut Q,
    ) -> Result<<Q::Fetch as FetchResource<'r, Q>>::Item, ECSError> {
        <Q::Fetch as FetchResource<'r, Q>>::fetch(self, claim)
    }
}

/// Query a resource by type for shared access
#[derive(Debug)]
pub struct ResQuery<T: Resource>(PhantomData<fn(T)>);

impl<T: Resource> Default for ResQuery<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource> ResQuery<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource> ResourceQuery for ResQuery<T> {
    type Fetch = ResFetch<T>;
}

/// Fetch for Res<'_, T>
pub struct ResFetch<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, ResQuery<T>> for ResFetch<T> {
    type Item = Res<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, _claim: &'r mut ResQuery<T>) -> Result<Self::Item, ECSError> {
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

impl<'a, T: Resource> ResourceAccess for Res<'a, T> {
    type Query = ResQuery<T>;
    type Fetch = ResFetch<T>;
}

/// Query a resource by type for exclusive access
#[derive(Debug)]
pub struct ResMutQuery<T: Resource>(PhantomData<fn(T)>);

impl<T: Resource> Default for ResMutQuery<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource> ResMutQuery<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource> ResourceQuery for ResMutQuery<T> {
    type Fetch = ResMutFetch<T>;
}

/// Fetch for ResMut<'_, T>
pub struct ResMutFetch<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, ResMutQuery<T>> for ResMutFetch<T> {
    type Item = ResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, _claim: &'r mut ResMutQuery<T>) -> Result<Self::Item, ECSError> {
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

impl<'a, T: Resource> ResourceAccess for ResMut<'a, T> {
    type Query = ResMutQuery<T>;
    type Fetch = ResMutFetch<T>;
}

/// Query resources of the same type by id for shared access
pub struct MultiResQuery<T: Resource>(Vec<ResourceId>, PhantomData<fn(T)>);

impl<T: Resource> Default for MultiResQuery<T> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T: Resource> MultiResQuery<T> {
    pub fn new(ids: Vec<ResourceId>) -> Self {
        Self(ids, PhantomData)
    }

    pub fn add_ids<I: IntoIterator<Item = ResourceId>>(&mut self, iter: I) {
        self.0.extend(iter);
    }

    pub fn try_add_tags<I>(&mut self, iter: I) -> Result<(), ECSError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let ids = iter
            .into_iter()
            .map(|tag| ResourceId::from_tag(tag.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        self.add_ids(ids);
        Ok(())
    }

    pub fn try_add_objects<I, K>(&mut self, iter: I) -> Result<(), ECSError>
    where
        I: IntoIterator,
        I::Item: AsRef<K>,
        K: Serialize,
    {
        let ids = iter
            .into_iter()
            .map(|obj| ResourceId::from_object(obj.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        self.add_ids(ids);
        Ok(())
    }

    pub fn iter(&self) -> Iter<'_, ResourceId> {
        self.0.iter()
    }
}

impl<T: Resource> fmt::Debug for MultiResQuery<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_tuple("MultiResQuery");
        self.0.iter().for_each(|x| {
            dbg.field(&x);
        });
        dbg.finish()
    }
}

impl<T: Resource> ResourceQuery for MultiResQuery<T> {
    type Fetch = MultiResFetch<T>;
}

/// Fetch for MultiRes<'_, T>.
pub struct MultiResFetch<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, MultiResQuery<T>> for MultiResFetch<T> {
    type Item = MultiRes<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, claims: &'r mut MultiResQuery<T>) -> Result<Self::Item, ECSError> {
        let resources = resources.get_with_ids::<T, _>(claims.0.iter())?;
        Ok(MultiRes(resources, claims))
    }
}

/// Shared borrow of multiple resources of the same type
pub struct MultiRes<'a, T: Resource>(ResourceMultiRead<'a, T>, &'a MultiResQuery<T>);

impl<'a, T: Resource> MultiRes<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn claim(&self) -> &MultiResQuery<T> {
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

impl<'a, T: Resource> ResourceAccess for MultiRes<'a, T> {
    type Query = MultiResQuery<T>;
    type Fetch = MultiResFetch<T>;
}

/// Query resources of the same type by id for exclusive access
pub struct MultiResMutQuery<T: Resource>(Vec<ResourceId>, PhantomData<fn(T)>);

impl<T: Resource> Default for MultiResMutQuery<T> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T: Resource> MultiResMutQuery<T> {
    pub fn new(ids: Vec<ResourceId>) -> Self {
        Self(ids, PhantomData)
    }

    pub fn add_ids<I: IntoIterator<Item = ResourceId>>(&mut self, iter: I) {
        self.0.extend(iter);
    }

    pub fn try_add_tags<I>(&mut self, iter: I) -> Result<(), ECSError>
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let ids = iter
            .into_iter()
            .map(|tag| ResourceId::from_tag(tag.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        self.add_ids(ids);
        Ok(())
    }

    pub fn try_add_objects<I, K>(&mut self, iter: I) -> Result<(), ECSError>
    where
        I: IntoIterator,
        I::Item: AsRef<K>,
        K: Serialize,
    {
        let ids = iter
            .into_iter()
            .map(|obj| ResourceId::from_object(obj.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        self.add_ids(ids);
        Ok(())
    }

    pub fn iter(&self) -> Iter<'_, ResourceId> {
        self.0.iter()
    }
}

impl<T: Resource> fmt::Debug for MultiResMutQuery<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_tuple("MultiResMutQuery");
        self.0.iter().for_each(|x| {
            dbg.field(&x);
        });
        dbg.finish()
    }
}

impl<T: Resource> ResourceQuery for MultiResMutQuery<T> {
    type Fetch = MultiResMutFetch<T>;
}

/// Fetch for MultiRes<'_, T>.
pub struct MultiResMutFetch<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, MultiResMutQuery<T>> for MultiResMutFetch<T> {
    type Item = MultiResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, claims: &'r mut MultiResMutQuery<T>) -> Result<Self::Item, ECSError> {
        let resources = resources.get_mut_with_ids::<T, _>(&claims.0)?;
        Ok(MultiResMut(resources, claims))
    }
}

/// Shared borrow of multiple resources of the same type
pub struct MultiResMut<'a, T: Resource>(ResourceMultiWrite<'a, T>, &'a MultiResMutQuery<T>);

impl<'a, T: Resource> MultiResMut<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn claim(&self) -> &MultiResMutQuery<T> {
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

impl<'a, T: Resource> ResourceAccess for MultiResMut<'a, T> {
    type Query = MultiResMutQuery<T>;
    type Fetch = MultiResMutFetch<T>;
}
