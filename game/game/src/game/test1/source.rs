use crate::{
    assets::{
        cooker::{CookingError, ModelCooker, Naming, PipelineCooker, TextureCooker},
        AssetError, AssetIO, AssetId, ContentHash, Url,
    },
    game::test1::Test1,
};

pub struct Source {
    pub source_id: AssetId,
    pub source_url: Url,
    pub test: Test1,
}

impl Source {
    pub async fn load(
        io: &AssetIO,
        source_id: &AssetId,
        source_url: &Url,
    ) -> Result<(Source, ContentHash), AssetError> {
        log::debug!("[{}] Downloading from {} ...", source_id, source_url);
        let data = io.download_binary(&source_url).await?;
        Self::load_from_data(source_id, source_url, &data).await
    }

    pub async fn load_from_data(
        source_id: &AssetId,
        source_url: &Url,
        data: &[u8],
    ) -> Result<(Source, ContentHash), AssetError> {
        let test = serde_json::from_slice::<Test1>(&data).map_err(|err| AssetError::load_failed(&source_url, err))?;
        log::trace!("[{}] descriptor: {:#?}", source_id, test);

        let source = Source {
            source_id: source_id.clone(),
            source_url: source_url.clone(),
            test,
        };
        let source_hash = ContentHash::from_bytes(&data);
        Ok((source, source_hash))
    }

    pub async fn cook<'a, C>(self, cooker: C) -> Result<Test1, CookingError>
    where
        C: PipelineCooker<'a> + TextureCooker<'a> + ModelCooker<'a>,
    {
        log::debug!("[{}] Compiling...", self.source_url);

        let Source { source_id, test, .. } = self;
        let Test1 { ty, pipeline } = test;

        log::debug!("[{}] Checking pipeline ({}) dependency...", source_id, pipeline);
        let pip_id = source_id
            .create_relative(&pipeline)
            .map_err(|err| CookingError::from_err(&source_id, err))?;
        let pipeline = cooker
            .cook_pipeline(pip_id, Naming::hard("pipeline", "pl"))
            .await?
            .to_string();

        Ok(Test1 { ty, pipeline })
    }
}
