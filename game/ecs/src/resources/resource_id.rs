use crate::{
    core::{error::ErrorString, ids::SmallStringId},
    ECSError,
};
use std::{
    collections::hash_map::DefaultHasher,
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
    sync::Arc,
};

pub type ResourceTag = SmallStringId<16>;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceId {
    Global,
    Tag(ResourceTag),
    Counter(usize),
    Binary(Vec<u8>),
}

impl ResourceId {
    pub fn from_counter(cnt: usize) -> Self {
        ResourceId::Counter(cnt)
    }

    pub fn from_tag(tag: &str) -> Result<Self, ECSError> {
        Ok(ResourceId::Tag(
            ResourceTag::from_str(tag).map_err(|err| ECSError::ResourceId(Arc::new(err)))?,
        ))
    }

    pub fn from_object<T>(obj: &T) -> Result<Self, ECSError>
    where
        T: serde::Serialize,
    {
        Ok(ResourceId::Binary(
            bincode::serialize(&obj).map_err(|err| ECSError::ResourceId(Arc::new(err)))?,
        ))
    }

    pub fn to_object<'a, T>(&'a self) -> Result<T, ECSError>
    where
        T: serde::Deserialize<'a>,
    {
        if let ResourceId::Binary(data) = self {
            bincode::deserialize::<T>(&data).map_err(|err| ECSError::ResourceId(Arc::new(err)))
        } else {
            Err(ECSError::ResourceId(Arc::new(ErrorString(format!("Not a binary id")))))
        }
    }
}

impl fmt::Debug for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hash = {
            let mut h = DefaultHasher::new();
            self.hash(&mut h);
            h.finish()
        };
        match self {
            ResourceId::Global => f.debug_tuple("Global").field(&hash).finish(),
            ResourceId::Tag(tag) => f.debug_tuple("Tag").field(&hash).field(tag).finish(),
            ResourceId::Counter(cnt) => f.debug_tuple("Counter").field(&hash).field(cnt).finish(),
            ResourceId::Binary(bin) => f.debug_tuple("Binary").field(&hash).field(&bin.len()).finish(),
        }
    }
}

impl AsRef<ResourceId> for ResourceId {
    #[inline]
    fn as_ref(&self) -> &ResourceId {
        self
    }
}
