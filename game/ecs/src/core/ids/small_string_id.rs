use crate::core::ids::IdError;
use serde::{de, ser};
use std::{
    fmt,
    marker::PhantomData,
    str::{self, FromStr},
};

/// An ID the requires no additional heap alloction.
#[derive(Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct SmallStringId<const N: usize> {
    inner: [u8; N],
}

impl<const N: usize> Default for SmallStringId<N> {
    fn default() -> Self {
        Self { inner: [0; N] }
    }
}

impl<const N: usize> SmallStringId<N> {
    pub fn as_str(&self) -> &str {
        let end = self.inner.iter().position(|&b| b == 0).unwrap_or(N);
        str::from_utf8(&self.inner[..end]).unwrap()
    }
}

impl<const N: usize> FromStr for SmallStringId<N> {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() <= N {
            let mut id = Self::default();
            id.inner[..s.len()].copy_from_slice(s.as_bytes());
            Ok(id)
        } else {
            Err(IdError::ParseError(s.to_owned()))
        }
    }
}

impl<const N: usize> fmt::Debug for SmallStringId<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SmallStringId").field(&self.as_str()).finish()
    }
}

impl<const N: usize> ser::Serialize for SmallStringId<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

struct SmallStringIdVisitor<const N: usize> {
    phantom: PhantomData<[u8; N]>,
}

impl<'de, const N: usize> de::Visitor<'de> for SmallStringIdVisitor<N> {
    type Value = SmallStringId<N>;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a string")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
        SmallStringId::from_str(v).map_err(de::Error::custom)
    }

    fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
        SmallStringId::from_str(&v).map_err(de::Error::custom)
    }
}

impl<'de, const N: usize> de::Deserialize<'de> for SmallStringId<N> {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(SmallStringIdVisitor { phantom: PhantomData })
    }
}
