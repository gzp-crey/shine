use crate::resources::{MultiResMutQuery, MultiResQuery, ResMutQuery, ResQuery, Resource, ResourceId};
use std::{any::TypeId, collections::HashSet};

/// Shared an unique resource requests
#[derive(Default, Debug)]
pub struct ResourceClaims {
    all_immutable: HashSet<(TypeId, ResourceId)>,
    all_mutable: HashSet<(TypeId, ResourceId)>,
}

impl ResourceClaims {
    fn store_immutable(&mut self, idx: (TypeId, ResourceId)) {
        assert!(self.all_mutable.get(&idx).is_none()); // claimed a resources both as mutable and immutable
        self.all_immutable.insert(idx);
    }

    fn store_mutable(&mut self, idx: (TypeId, ResourceId)) {
        assert!(self.all_immutable.get(&idx).is_none()); // claimed a resources both as mutable and immutable
        assert!(self.all_mutable.get(&idx).is_none()); // claimed a resources multiple times for mutation
        self.all_mutable.insert(idx);
    }

    pub fn add_immutable<T, I>(&mut self, immutable: I)
    where
        T: Resource,
        I: IntoIterator<Item = ResourceId>,
    {
        immutable
            .into_iter()
            .for_each(|x| self.store_immutable((TypeId::of::<T>(), x)));
    }

    pub fn add_mutable<T, I>(&mut self, mutable: I)
    where
        T: Resource,
        I: IntoIterator<Item = ResourceId>,
    {
        mutable
            .into_iter()
            .for_each(|x| self.store_mutable((TypeId::of::<T>(), x)));
    }

    pub fn add_claim<C: ResourceClaim>(&mut self, claim: &C) {
        claim.add_claim(self);
    }
}

pub trait ResourceClaim {
    fn add_claim(&self, claims: &mut ResourceClaims);
}

impl<T: Resource> ResourceClaim for ResQuery<T> {
    fn add_claim(&self, claims: &mut ResourceClaims) {
        claims.add_immutable::<T, _>(Some(ResourceId::Global))
    }
}

impl<T: Resource> ResourceClaim for ResMutQuery<T> {
    fn add_claim(&self, claims: &mut ResourceClaims) {
        claims.add_mutable::<T, _>(Some(ResourceId::Global))
    }
}

impl<T: Resource> ResourceClaim for MultiResQuery<T> {
    fn add_claim(&self, claims: &mut ResourceClaims) {
        claims.add_immutable::<T, _>(self.iter().cloned())
    }
}

impl<T: Resource> ResourceClaim for MultiResMutQuery<T> {
    fn add_claim(&self, claims: &mut ResourceClaims) {
        claims.add_mutable::<T, _>(self.iter().cloned())
    }
}
