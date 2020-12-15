#[cfg(feature = "cook")]
use crate::{
    assets::{
        cooker::{CookingError, Naming, PipelineCooker},
        io::HashableContent,
        AssetError, AssetIO, AssetId, Url,
    },
    game::test1::{Test1, Test1Type},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Descriptor {
    pub pipeline: String,
}

pub struct Source {
    pub source_id: AssetId,
    pub source_url: Url,
    pub test: Descriptor,
}

impl Source {
    pub async fn load(io: &AssetIO, source_id: &AssetId, source_url: &Url) -> Result<(Self, String), AssetError> {
        log::debug!("[{}] Downloading from {} ...", source_id.as_str(), source_url.as_str());
        let data = io.download_binary(&source_url).await?;
        Self::load_from_data(source_id, source_url, &data).await
    }

    pub async fn load_from_data(
        source_id: &AssetId,
        source_url: &Url,
        data: &[u8],
    ) -> Result<(Self, String), AssetError> {
        let test = serde_json::from_slice::<Descriptor>(&data)
            .map_err(|err| AssetError::load_failed(source_url.as_str(), err))?;
        log::trace!("[{}] descriptor: {:#?}", source_id.as_str(), test);

        let source = Source {
            source_id: source_id.clone(),
            source_url: source_url.clone(),
            test,
        };
        let source_hash = data.content_hash();
        Ok((source, source_hash))
    }

    pub async fn cook<'a, C: PipelineCooker<'a>>(self, cooker: C) -> Result<Test1, CookingError> {
        log::debug!("[{}] Compiling...", self.source_url.as_str());

        let Source { source_id, test, .. } = self;
        let Descriptor { pipeline } = test;

        log::debug!(
            "[{}] Checking pipeline ({}) dependency...",
            source_id.as_str(),
            pipeline
        );
        let pip_id = source_id
            .create_relative(&pipeline)
            .map_err(|err| CookingError::from_err(source_id.as_str(), err))?;
        let pipeline = cooker.cook_pipeline(pip_id, Naming::Soft).await?.to_string();

        Ok(Test1 {
            ty: Test1Type::Test1,
            pipeline,
        })
    }
}
