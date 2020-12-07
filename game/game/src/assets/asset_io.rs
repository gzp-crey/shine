use crate::assets::io::AssetLowIO;
use crate::assets::{AssetError, Url};
use std::collections::HashMap;
use std::sync::Arc;

struct Inner {
    io: AssetLowIO,
    virtual_schemes: HashMap<String, Url>,
}

#[derive(Clone)]
pub struct AssetIO {
    inner: Arc<Inner>,
}

impl AssetIO {
    pub fn new(virtual_schemes: HashMap<String, Url>) -> Result<AssetIO, AssetError> {
        Ok(AssetIO {
            inner: Arc::new(Inner {
                io: AssetLowIO::new()?,
                virtual_schemes,
            }),
        })
    }

    pub fn resolve_virtual_scheme(&self, url: &Url) -> Result<Url, AssetError> {
        let scheme = url.scheme().to_owned();
        if let Some(base) = self.inner.virtual_schemes.get(&scheme) {
            Ok(url.replace_virtual_scheme(base)?)
        } else {
            Ok(url.clone())
        }
    }

    pub async fn download_etag(&self, url: &Url) -> Result<String, AssetError> {
        let url = self.resolve_virtual_scheme(url)?;
        self.inner.io.download_etag(&url).await
    }

    pub async fn download_binary(&self, url: &Url) -> Result<Vec<u8>, AssetError> {
        let url = self.resolve_virtual_scheme(url)?;
        self.inner.io.download_binary(&url).await
    }

    pub async fn download_string(&self, url: &Url) -> Result<String, AssetError> {
        let url = self.resolve_virtual_scheme(url)?;
        String::from_utf8(self.inner.io.download_binary(&url).await?)
            .map_err(|err| AssetError::load_failed(url.as_str(), err))
    }

    pub async fn upload_binary(&self, url: &Url, data: &[u8]) -> Result<(), AssetError> {
        let url = self.resolve_virtual_scheme(&url)?;
        self.inner.io.upload_binary(&url, data).await
    }

    pub async fn upload_string(&self, url: &Url, data: &str) -> Result<(), AssetError> {
        let url = self.resolve_virtual_scheme(url)?;
        self.inner.io.upload_binary(&url, data.as_bytes()).await
    }
}
