#[cfg(feature = "cook")]
use crate::{
    assets::{io::HashableContent, AssetError, AssetIO, CookingError, Url},
    game::test1::Test1,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Descriptor {
    pub pipeline: String,
}

/// Serialized test
#[derive(Serialize, Deserialize)]
pub struct Source {
    pub source_url: Url,
    pub test: Descriptor,
}

impl Source {
    pub async fn load(io: &AssetIO, source_url: &Url) -> Result<(Self, String), AssetError> {
        log::debug!("[{}] Downloading ...", source_url.as_str());
        let data = io.download_binary(&source_url).await?;
        Self::load_from_data(source_url, &data).await
    }

    pub async fn load_from_data(source_url: &Url, data: &[u8]) -> Result<(Self, String), AssetError> {
        let test = serde_json::from_slice::<Descriptor>(&data)
            .map_err(|err| AssetError::load_failed(source_url.as_str(), err))?;
        log::trace!("[{}] Test:\n{:#?}", source_url.as_str(), test);

        let source = Source {
            source_url: source_url.clone(),
            test,
        };
        let source_hash = data.content_hash();
        Ok((source, source_hash))
    }

    pub async fn cook(self) -> Result<Test1, CookingError> {
        log::debug!("[{}] Compiling...", self.source_url.as_str());

        let Source { test, .. } = self;
        let Descriptor { pipeline } = test;

        Ok(Test1 { pipeline })
    }
}
