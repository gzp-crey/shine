use crate::assets::{Url, UrlError};

/// Id of an asset used to identify asset in the container
#[derive(Debug, Clone)]
pub struct AssetId {
    inner: String,
}

impl AssetId {
    pub fn new<S: ToString>(id: S) -> Result<AssetId, UrlError> {
        let id = id.to_string();
        if id.chars().any(|c| c == '?' || c == '&') {
            Err(UrlError::InvalidDomainCharacter)
        } else if id.starts_with("./") {
            Err(UrlError::RelativeUrlWithoutBase)
        } else {
            Ok(AssetId { inner: id })
        }
    }

    pub fn create_relative(&self, id: &str) -> Result<AssetId, UrlError> {
        if let Some(id) = id.strip_prefix("./") {
            let (folder, _) = self.split_folder();
            if let Some(folder) = folder {
                AssetId::new(&format!("{}/{}", folder, id))
            } else {
                AssetId::new(id)
            }
        } else {
            AssetId::new(id)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    pub fn to_string(&self) -> String {
        self.inner.clone()
    }

    pub fn into_string(self) -> String {
        self.inner
    }

    pub fn extension(&self) -> &str {
        let (_, file) = self.split_folder();
        let mut parts = file.splitn(2, '.');
        let first = parts.next();
        let second = parts.next();
        first.and(second).unwrap_or("")
    }

    pub fn set_extension(&self, ext: &str) -> Result<AssetId, UrlError> {
        let (folder, file) = self.split_folder();
        let mut parts = file.splitn(2, '.');
        let first = parts.next();
        let file = first.unwrap_or("");

        let inner = if let Some(folder) = folder {
            format!("{}/{}.{}", folder, file, ext)
        } else {
            format!("{}.{}", file, ext)
        };

        AssetId::new(&inner)
    }

    pub fn split_folder(&self) -> (Option<&str>, &str) {
        let mut parts = self.inner.rsplitn(2, '/');
        let first = parts.next();
        let second = parts.next();
        (first.and(second), first.or(second).unwrap_or(""))
    }

    pub fn to_url(&self, base: &Url) -> Result<Url, UrlError> {
        base.join(&self.inner)
    }
}
