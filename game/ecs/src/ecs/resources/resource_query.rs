use crate::{
    core::ids::IdError,
    ecs::{
        resources::{
            Resource, ResourceClaim, ResourceHandle, ResourceId, ResourceRead, ResourceTag, ResourceWrite, Resources,
            TaggedResourceRead, TaggedResourceWrite,
        },
        ECSError,
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
    fn into_claim(&self) -> Result<ResourceClaim, ECSError>;
}

pub trait ResourceQuery {
    type Fetch: for<'a> FetchResource<'a, Self::Claim>;
    type Claim: Default + IntoResourceClaim;

    fn default_claims() -> ResourceClaim;
}

pub trait FetchResource<'a, Claim> {
    type Item;

    fn fetch<'r: 'a>(resources: &'r Resources, extra_claim: &'r Claim) -> Result<Self::Item, ECSError>;
}

#[derive(Debug)]
pub struct ResClaim<T: Resource>(PhantomData<fn(T)>);

impl<T: Resource> Default for ResClaim<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Resource> IntoResourceClaim for ResClaim<T> {
    fn into_claim(&self) -> Result<ResourceClaim, ECSError> {
        Ok(ResourceClaim::none())
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
        ResourceClaim::new(Some(ResourceHandle::new::<T>(ResourceId::Global)), None)
    }
}

pub struct FetchResourceRead<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, ResClaim<T>> for FetchResourceRead<T> {
    type Item = Res<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, _extra_claim: &'r ResClaim<T>) -> Result<Self::Item, ECSError> {
        Ok(Res(resources.get::<T>()?))
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
    fn into_claim(&self) -> Result<ResourceClaim, ECSError> {
        Ok(ResourceClaim::none())
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
        ResourceClaim::new(None, Some(ResourceHandle::new::<T>(ResourceId::Global)))
    }
}

pub struct FetchResourceWrite<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, ResMutClaim<T>> for FetchResourceWrite<T> {
    type Item = ResMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, _extra_claim: &'r ResMutClaim<T>) -> Result<Self::Item, ECSError> {
        Ok(ResMut(resources.get_mut::<T>()?))
    }
}

/// List of resource tags for the shared borrower, [Tag]
pub struct TagClaim<T: Resource>(Vec<ResourceTag>, PhantomData<fn(T)>);

impl<T: Resource> IntoResourceClaim for TagClaim<T> {
    fn into_claim(&self) -> Result<ResourceClaim, ECSError> {
        let immutable = self
            .0
            .iter()
            .map(|c| ResourceHandle::new::<T>(ResourceId::Tag(c.clone())));
        Ok(ResourceClaim::new(immutable, None))
    }
}

impl<T: Resource> Default for TagClaim<T> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T: Resource> Deref for TagClaim<T> {
    type Target = Vec<ResourceTag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Resource> DerefMut for TagClaim<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Resource> fmt::Debug for TagClaim<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_tuple("TagClaim");
        self.0.iter().for_each(|x| {
            dbg.field(&x.as_str());
        });
        dbg.finish()
    }
}

impl<'a, 'b, T: Resource> TryFrom<&'a [&'b str]> for TagClaim<T> {
    type Error = IdError;

    fn try_from(value: &'a [&'b str]) -> Result<Self, Self::Error> {
        let tags = value
            .into_iter()
            .map(|tag| ResourceTag::from_str(tag))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(tags, PhantomData))
    }
}

impl<'a, 'b, T: Resource, const N: usize> TryFrom<&'a [&'b str; N]> for TagClaim<T> {
    type Error = IdError;

    fn try_from(value: &'a [&'b str; N]) -> Result<Self, Self::Error> {
        let tags = value
            .into_iter()
            .map(|tag| ResourceTag::from_str(tag))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(tags, PhantomData))
    }
}

/// Shared borrow of multiple tagged resources.
pub struct Tag<'a, T: Resource>(TaggedResourceRead<'a, T>, &'a TagClaim<T>);

impl<'a, T: Resource> Tag<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn claim(&self) -> &TagClaim<T> {
        &self.1
    }

    pub fn position_by_tag(&self, tag: &ResourceTag) -> Option<usize> {
        self.1.iter().position(|x| x == tag)
    }
}

impl<'a, T: Resource> Index<usize> for Tag<'a, T> {
    type Output = T;

    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

impl<'a, T: Resource> ResourceQuery for Tag<'a, T> {
    type Claim = TagClaim<T>;
    type Fetch = FetchTaggedResourceRead<T>;

    fn default_claims() -> ResourceClaim {
        ResourceClaim::none()
    }
}

pub struct FetchTaggedResourceRead<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, TagClaim<T>> for FetchTaggedResourceRead<T> {
    type Item = Tag<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, extra_claim: &'r TagClaim<T>) -> Result<Self::Item, ECSError> {
        let resources = resources.get_with_tags::<T, _>(extra_claim.iter())?;
        Ok(Tag(resources, extra_claim))
    }
}

/// List of resource tags for the unique borrower, [TagMut]
pub struct TagMutClaim<T: Resource>(Vec<ResourceTag>, PhantomData<fn(T)>);

impl<T: Resource> IntoResourceClaim for TagMutClaim<T> {
    fn into_claim(&self) -> Result<ResourceClaim, ECSError> {
        let mutable = self
            .0
            .iter()
            .map(|c| ResourceHandle::new::<T>(ResourceId::Tag(c.clone())));
        Ok(ResourceClaim::new(None, mutable))
    }
}

impl<T: Resource> Default for TagMutClaim<T> {
    fn default() -> Self {
        Self(Vec::new(), PhantomData)
    }
}

impl<T: Resource> Deref for TagMutClaim<T> {
    type Target = Vec<ResourceTag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Resource> DerefMut for TagMutClaim<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Resource> fmt::Debug for TagMutClaim<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg = f.debug_tuple("TagMutClaim");
        self.0.iter().for_each(|x| {
            dbg.field(&x.as_str());
        });
        dbg.finish()
    }
}

impl<'a, 'b, T: Resource> TryFrom<&'a [&'b str]> for TagMutClaim<T> {
    type Error = IdError;

    fn try_from(value: &'a [&'b str]) -> Result<Self, Self::Error> {
        let tags = value
            .into_iter()
            .map(|tag| ResourceTag::from_str(tag))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(tags, PhantomData))
    }
}

/// Unique borrow of multiple tagged resources.
pub struct TagMut<'a, T: Resource>(TaggedResourceWrite<'a, T>, &'a TagMutClaim<T>);

impl<'a, T: Resource> TagMut<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn claim(&self) -> &TagMutClaim<T> {
        &self.1
    }

    pub fn position_by_tag(&self, tag: &ResourceTag) -> Option<usize> {
        self.1.iter().position(|x| x == tag)
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
    type Fetch = FetchTaggedResourceWrite<T>;
    type Claim = TagMutClaim<T>;

    fn default_claims() -> ResourceClaim {
        ResourceClaim::none()
    }
}

pub struct FetchTaggedResourceWrite<T: Resource>(PhantomData<T>);

impl<'a, T: Resource> FetchResource<'a, TagMutClaim<T>> for FetchTaggedResourceWrite<T> {
    type Item = TagMut<'a, T>;

    fn fetch<'r: 'a>(resources: &'r Resources, extra_claim: &'r TagMutClaim<T>) -> Result<Self::Item, ECSError> {
        let resources = resources.get_mut_with_tags::<T, _>(extra_claim.iter())?;
        Ok(TagMut(resources, extra_claim))
    }
}
