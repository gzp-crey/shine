use crate::core::ids::IdError;
use std::{
    fmt,
    str::{self, FromStr},
};

/// An ID the requires no additional heap alloction.
#[derive(Clone, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub struct SmallStringId<const N: usize> {
    inner: [u8; N],
}

impl<const N: usize> SmallStringId<N> {
    pub fn as_str(&self) -> &str {
        let end = self.inner.iter().position(|&b| b == 0).unwrap_or(N);
        str::from_utf8(&self.inner[..end]).unwrap()
    }
}

impl<const N: usize> Default for SmallStringId<N> {
    fn default() -> Self {
        Self { inner: [0; N] }
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
