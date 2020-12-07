use crate::{
    resources::{
        Resource, ResourceId, ResourceMultiRead, ResourceMultiWrite, ResourceRead, ResourceTag, ResourceWrite,
        Resources,
    },
    scheduler::ResourceClaim,
    ECSError,
};
use std::{
    any::TypeId,
    convert::TryFrom,
    fmt,
    marker::PhantomData,
    ops::{Deref, DerefMut, Index, IndexMut},
    unreachable,
};

pub trait IntoResourceClaim {
    fn into_claim(&self) -> ResourceClaim;
}

pub trait ResourceQuery {
    type Fetch: for<'a> FetchResource<'a, Self::Claim>;
    type Claim: Default + IntoResourceClaim;
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
    type Fetch = ResMutFetch<T>;
}

/// Claims for TagRes<'_, T>. A list of resource tags.
pub struct TagResClaim<T: Resource>(Vec<ResourceId>, PhantomData<fn(T)>);

impl<T: Resource> Default for TagResClaim<T> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T: Resource> IntoResourceClaim for TagResClaim<T> {
    fn into_claim(&self) -> ResourceClaim {
        let immutable = self.0.iter().map(|c| (TypeId::of::<T>(), c.clone()));
        ResourceClaim::new(immutable, None)
    }
}

impl<T: Resource> fmt::Debug for TagResClaim<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_tuple("TagResClaim");
        self.0.iter().for_each(|x| match x {
            ResourceId::Tag(x) => {
                dbg.field(&x.as_str());
            }
            _ => unreachable!(),
        });
        dbg.finish()
    }
}

impl<'a, 'b, T: Resource> TryFrom<&'a [&'b str]> for TagResClaim<T> {
    type Error = ECSError;

    fn try_from(value: &'a [&'b str]) -> Result<Self, Self::Error> {
        let tags = value
            .into_iter()
            .map(|tag| ResourceId::from_tag(tag))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(tags, PhantomData))
    }
}

impl<'a, 'b, T: Resource, const N: usize> TryFrom<&'a [&'b str; N]> for TagResClaim<T> {
    type Error = ECSError;

    fn try_from(value: &'a [&'b str; N]) -> Result<Self, Self::Error> {
        let tags = value
            .into_iter()
            .map(|tag| ResourceId::from_tag(tag))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(tags, PhantomData))
    }
}

/// Fetch for TagRes<'_, T>.
pub struct TagResFetch<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, TagResClaim<T>> for TagResFetch<T> {
    type Item = Tag<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, claims: &'r TagResClaim<T>) -> Result<Self::Item, ECSError> {
        let resources = resources.get_with_ids::<T, _>(&claims.0)?;
        Ok(Tag(resources, claims))
    }
}

/// Shared borrow of multiple tagged resources.
pub struct Tag<'a, T: Resource>(ResourceMultiRead<'a, T>, &'a TagResClaim<T>);

impl<'a, T: Resource> Tag<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn claim(&self) -> &TagResClaim<T> {
        &self.1
    }

    pub fn position_by_tag(&self, tag: &ResourceTag) -> Option<usize> {
        (self.1).0.iter().position(|x| match x {
            ResourceId::Tag(t) => t == tag,
            _ => unreachable!(),
        })
    }
}

impl<'a, T: Resource> Index<usize> for Tag<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

impl<'a, T: Resource> ResourceQuery for Tag<'a, T> {
    type Claim = TagResClaim<T>;
    type Fetch = TagResFetch<T>;
}

/// Claims for TagMutRes<'_, T>. A list of resource tags.
pub struct TagMutResClaim<T: Resource>(Vec<ResourceId>, PhantomData<fn(T)>);

impl<T: Resource> Default for TagMutResClaim<T> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T: Resource> IntoResourceClaim for TagMutResClaim<T> {
    fn into_claim(&self) -> ResourceClaim {
        let mutable = self.0.iter().map(|c| (TypeId::of::<T>(), c.clone()));
        ResourceClaim::new(None, mutable)
    }
}

impl<T: Resource> fmt::Debug for TagMutResClaim<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_tuple("TagMutResClaim");
        self.0.iter().for_each(|x| match x {
            ResourceId::Tag(x) => {
                dbg.field(&x.as_str());
            }
            _ => unreachable!(),
        });
        dbg.finish()
    }
}

impl<'a, 'b, T: Resource> TryFrom<&'a [&'b str]> for TagMutResClaim<T> {
    type Error = ECSError;

    fn try_from(value: &'a [&'b str]) -> Result<Self, Self::Error> {
        let tags = value
            .into_iter()
            .map(|tag| ResourceId::from_tag(tag))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(tags, PhantomData))
    }
}

impl<'a, 'b, T: Resource, const N: usize> TryFrom<&'a [&'b str; N]> for TagMutResClaim<T> {
    type Error = ECSError;

    fn try_from(value: &'a [&'b str; N]) -> Result<Self, Self::Error> {
        let tags = value
            .into_iter()
            .map(|tag| ResourceId::from_tag(tag))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(tags, PhantomData))
    }
}

/// Fetch for TagRes<'_, T>.
pub struct TagResMutFetch<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, TagMutResClaim<T>> for TagResMutFetch<T> {
    type Item = TagMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, claims: &'r TagMutResClaim<T>) -> Result<Self::Item, ECSError> {
        let resources = resources.get_mut_with_ids::<T, _>(&claims.0)?;
        Ok(TagMut(resources, claims))
    }
}

/// Shared borrow of multiple tagged resources.
pub struct TagMut<'a, T: Resource>(ResourceMultiWrite<'a, T>, &'a TagMutResClaim<T>);

impl<'a, T: Resource> TagMut<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn claim(&self) -> &TagMutResClaim<T> {
        &self.1
    }

    pub fn position_by_tag(&self, tag: &ResourceTag) -> Option<usize> {
        (self.1).0.iter().position(|x| match x {
            ResourceId::Tag(t) => t == tag,
            _ => unreachable!(),
        })
    }
}

impl<'a, T: Resource> Index<usize> for TagMut<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

impl<'a, T: Resource> IndexMut<usize> for TagMut<'a, T> {
    fn index_mut(&mut self, idx: usize) -> &mut T {
        &mut self.0[idx]
    }
}

impl<'a, T: Resource> ResourceQuery for TagMut<'a, T> {
    type Claim = TagMutResClaim<T>;
    type Fetch = TagResMutFetch<T>;
}
