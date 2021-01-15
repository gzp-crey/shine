use crate::resources::{Resource, ResourceId};
use std::{any::TypeId, collections::HashSet};

#[derive(Default, Debug)]
pub struct ResourceClaim {
    pub immutable: Vec<(TypeId, ResourceId)>,
    pub mutable: Vec<(TypeId, ResourceId)>,
}

impl ResourceClaim {
    pub fn none() -> Self {
        Self {
            immutable: Vec::new(),
            mutable: Vec::new(),
        }
    }

    pub fn new<I1, I2>(immutable: I1, mutable: I2) -> Self
    where
        I1: IntoIterator<Item = (TypeId, ResourceId)>,
        I2: IntoIterator<Item = (TypeId, ResourceId)>,
    {
        Self {
            immutable: immutable.into_iter().collect(),
            mutable: mutable.into_iter().collect(),
        }
    }
}

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

    pub fn add_claim(&mut self, claim: ResourceClaim) {
        let ResourceClaim { immutable, mutable } = claim;
        immutable.into_iter().for_each(|x| self.store_immutable(x));
        mutable.into_iter().for_each(|x| self.store_mutable(x));
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

    /*pub fn is_claimed_immutable(&self, id: &(TypeId, ResourceId)) -> bool {
        self.all_immutable.contains(&id)
    }

    pub fn is_claimed_mutable(&self, id: &(TypeId, ResourceId)) -> bool {
        self.all_mutable.contains(&id)
    }*/
}
