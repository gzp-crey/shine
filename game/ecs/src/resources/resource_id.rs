use crate::{
    core::{error::ErrorString, ids::SmallStringId},
    ECSError,
};
use std::str::FromStr;

pub type ResourceTag = SmallStringId<16>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
            ResourceTag::from_str(tag).map_err(|err| ECSError::ResourceId(err.into()))?,
        ))
    }

    pub fn from_object<T>(obj: &T) -> Result<Self, ECSError>
    where
        T: serde::Serialize,
    {
        Ok(ResourceId::Binary(
            bincode::serialize(&obj).map_err(|err| ECSError::ResourceId(err.into()))?,
        ))
    }

    pub fn to_object<'a, T>(&'a self) -> Result<T, ECSError>
    where
        T: serde::Deserialize<'a>,
    {
        if let ResourceId::Binary(data) = self {
            bincode::deserialize::<T>(&data).map_err(|err| ECSError::ResourceId(err.into()))
        } else {
            Err(ECSError::ResourceId(Box::new(ErrorString(
                "Not a binary id".to_string(),
            ))))
        }
    }
}
