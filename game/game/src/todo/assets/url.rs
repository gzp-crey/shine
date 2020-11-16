use std::path::PathBuf;
pub use url::ParseError as UrlError;
pub use url::Position;

///A wrapper around url to make it more asset-io oriented
#[derive(Debug, Clone)]
pub struct Url {
    inner: url::Url,
}

impl Url {
    pub fn parse(input: &str) -> Result<Self, UrlError> {
        Ok(Url {
            inner: url::Url::parse(input)?,
        })
    }

    /*pub fn from_base_or_current(base: &Url, current: &Url, input: &str) -> Result<Self, UrlError> {
        if input.starts_with('/') {
            // input is relative to the base
            base.join(input)
        } else {
            // input is relative to the current
            current.to_folder().and_then(|url| url.join(input))
        }
    }*/

    pub fn to_folder(&self) -> Result<Url, UrlError> {
        let path = &self.inner[url::Position::BeforeHost..url::Position::AfterPath];
        let mut parts = path.rsplitn(2, '/');
        let first = parts.next();
        let second = parts.next();
        let folder = first.and(second).unwrap_or("");

        Url::parse(&format!(
            "{}{}/{}",
            &self.inner[..url::Position::BeforeHost],
            folder,
            &self.inner[url::Position::AfterPath..]
        ))
    }

    pub fn path(&self) -> &str {
        &self.inner[url::Position::BeforePath..url::Position::AfterPath]
    }

    pub fn relative_path(&self, base: &Url) -> Option<&str> {
        let path = &self.inner[..url::Position::AfterPath];
        let prefix = base.as_str();
        path.strip_prefix(prefix)
    }

    pub fn to_file_path(&self) -> PathBuf {
        PathBuf::from(&self.inner[url::Position::BeforeHost..url::Position::AfterPath])
    }

    pub fn to_file_folder(&self) -> PathBuf {
        let path = &self.inner[url::Position::BeforeHost..url::Position::AfterPath];
        let mut parts = path.rsplitn(2, '/');
        let first = parts.next();
        let second = parts.next();
        let folder = first.and(second).unwrap_or("");
        PathBuf::from(folder)
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    pub fn scheme(&self) -> &str {
        self.inner.scheme()
    }

    pub fn set_scheme(&self, scheme: &str) -> Result<Url, UrlError> {
        Url::parse(&format!("{}{}", scheme, &self.inner[url::Position::AfterScheme..]))
    }

    pub fn replace_virtual_scheme(&self, base: &Url) -> Result<Url, UrlError> {
        let prefix = &base.inner[..url::Position::AfterPath];
        let postfix = &base.inner[url::Position::BeforeQuery..];
        let path = &self.inner[url::Position::BeforeHost..url::Position::AfterPath];
        let query = &self.inner[url::Position::BeforeQuery..];
        match (postfix.is_empty(), query.is_empty()) {
            (true, true) => Url::parse(&format!("{}{}", prefix, path)),
            (true, false) => Url::parse(&format!("{}{}?{}", prefix, path, query)),
            (false, true) => Url::parse(&format!("{}{}?{}", prefix, path, postfix)),
            (false, false) => Url::parse(&format!("{}{}?{}&{}", prefix, path, query, postfix)),
        }
    }

    pub fn extension(&self) -> &str {
        let path = &self.inner[url::Position::BeforeHost..url::Position::AfterPath];
        let mut parts = path.rsplitn(2, '.');
        let first = parts.next();
        let second = parts.next();
        second.and(first).unwrap_or("")
    }

    pub fn set_extension(&self, ext: &str) -> Result<Url, UrlError> {
        let path = &self.inner[url::Position::BeforeHost..url::Position::AfterPath];
        let mut parts = path.rsplitn(2, '.');
        let first = parts.next();
        let second = parts.next();
        let base = second.or(first).unwrap_or("");

        Url::parse(&format!(
            "{}{}.{}{}",
            &self.inner[..url::Position::BeforeHost],
            base,
            ext,
            &self.inner[url::Position::AfterPath..]
        ))
    }

    pub fn join(&self, path: &str) -> Result<Url, UrlError> {
        Url::parse(&format!(
            "{}{}{}",
            &self.inner[..url::Position::AfterPath],
            path,
            &self.inner[url::Position::AfterPath..]
        ))
    }
}

mod serde_ser_de {
    use super::*;
    use serde::de::{self, Deserializer, Visitor};
    use serde::ser::Serializer;
    use serde::{Deserialize, Serialize};
    use std::fmt;

    impl Serialize for Url {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(self.as_str())
        }
    }

    struct UrlVisitor;

    impl<'de> Visitor<'de> for UrlVisitor {
        type Value = Url;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("An url was excpected")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Url::parse(value).map_err(|err| E::custom(format!("Failed to parse url: {}", err)))
        }
    }

    impl<'de> Deserialize<'de> for Url {
        fn deserialize<D>(deserializer: D) -> Result<Url, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_str(UrlVisitor)
        }
    }
}
