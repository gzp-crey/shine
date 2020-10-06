use crate::resources::ResourceIndex;
use std::collections::HashSet;

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
    pub fn none() -> Self {
        Self {
            immutable: Vec::new(),
            mutable: Vec::new(),
        }
    }

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

    pub fn add_claim(&mut self, claim: ResourceClaim) {
        let ResourceClaim { immutable, mutable } = claim;
        immutable.into_iter().for_each(|x| self.store_immutable(x));
        mutable.into_iter().for_each(|x| self.store_mutable(x));
    }

    pub fn is_claimed_immutable(&self, id: &ResourceIndex) -> bool {
        self.all_immutable.contains(&id)
    }

    pub fn is_claimed_mutable(&self, id: &ResourceIndex) -> bool {
        self.all_mutable.contains(&id)
    }
}
