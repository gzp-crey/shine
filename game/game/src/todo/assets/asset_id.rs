use crate::assets::{Url, UrlError};

/// Id of an asset used to identify asset in the container
#[derive(Debug, Clone)]
pub struct AssetId {
    inner: String,
}

impl AssetId {
    pub fn new(id: &str) -> Result<AssetId, UrlError> {
        if id.chars().any(|c| c == '?' || c == '&') {
            Err(UrlError::InvalidDomainCharacter)
        } else {
            Ok(AssetId { inner: id.to_owned() })
        }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    pub fn extension(&self) -> &str {
        let mut parts = self.inner.rsplitn(2, '.');
        let first = parts.next();
        let second = parts.next();
        second.and(first).unwrap_or("")
    }

    pub fn set_extension(&self, ext: &str) -> Result<AssetId, UrlError> {
        let mut parts = self.inner.rsplitn(2, '.');
        let first = parts.next();
        let second = parts.next();
        let base = second.or(first).unwrap_or("");

        Ok(AssetId {
            inner: format!("{}.{}", base, ext),
        })
    }

    pub fn to_absolute_id<'a, 'u>(&'a self, base: &'u Url, current_base: &'u Url) -> Result<AssetId, UrlError> {
        if self.inner.starts_with('/') {
            Ok(self.clone())
        } else {
            let absolute = current_base.join(&self.inner)?;
            let relative = absolute.relative_path(base).ok_or(UrlError::RelativeUrlWithoutBase)?;
            Ok(AssetId::new(relative)?)
        }
    }

    pub fn to_url(&self, base: &Url) -> Result<Url, UrlError> {
        base.join(&self.inner)
    }
}
