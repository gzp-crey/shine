#[cfg(feature = "cook")]
use crate::assets::{
    cooker::{CookingError, Naming, ShaderCooker},
    AssetError, AssetIO, AssetId, ContentHash, CookedPipeline, PipelineDescriptor, Url,
};

pub struct PipelineSource {
    pub source_id: AssetId,
    pub source_url: Url,
    pub descriptor: PipelineDescriptor,
}

impl PipelineSource {
    pub async fn load(
        io: &AssetIO,
        source_id: &AssetId,
        source_url: &Url,
    ) -> Result<(PipelineSource, ContentHash), AssetError> {
        log::debug!("[{}] Downloading from {} ...", source_id.as_str(), source_url.as_str());
        let data = io.download_binary(&source_url).await?;

        let pipeline = serde_json::from_slice::<PipelineDescriptor>(&data)
            .map_err(|err| AssetError::load_failed(source_id.as_str(), err))?;
        log::trace!("[{}] Pipeline:\n{:#?}", source_id.as_str(), pipeline);

        let source = PipelineSource {
            source_id: source_id.clone(),
            source_url: source_url.clone(),
            descriptor: pipeline,
        };
        let source_hash = ContentHash::from_bytes(&data);
        Ok((source, source_hash))
    }

    pub async fn cook<'a, C: ShaderCooker<'a>>(self, cookers: C) -> Result<CookedPipeline, CookingError> {
        log::debug!("[{}] Compiling...", self.source_id.as_str());

        let PipelineSource {
            source_id,
            mut descriptor,
            ..
        } = self;

        log::trace!("[{}] Pipeline descriptor: ({:#?})", source_id.as_str(), descriptor);

        // perform some consistency check
        /*for scope in [
            PipelineUniformScope::Auto,
            PipelineUniformScope::Global,
            PipelineUniformScope::Local,
        ]
        .iter()
        {
            let layout = descriptor.get_uniform_layout(*scope)?;
            log::trace!(
                "[{}] Uniform group({:?}) layout:\n{:#?}",
                source_url.as_str(),
                scope,
                layout
            );
        }*/

        // cook dependencies
        {
            let vs = &mut descriptor.vertex_stage;
            log::debug!(
                "[{}] Checking vertex shader ({}) dependency...",
                source_id.as_str(),
                vs.shader
            );
            let id = source_id
                .create_relative(&vs.shader)
                .map_err(|err| CookingError::from_err(source_id.as_str(), err))?;
            if id.extension() != "vs" {
                return Err(CookingError::from_err(
                    source_id.as_str(),
                    AssetError::UnsupportedFormat(id.extension().to_owned()),
                ));
            }
            vs.shader = cookers.cook_shader(id, Naming::hard("shader", "vs")).await?.to_string();
        }

        {
            let fs = &mut descriptor.fragment_stage;
            log::debug!(
                "[{}] Checking fragment shader ({}) dependency...",
                source_id.as_str(),
                fs.shader
            );
            let id = source_id
                .create_relative(&fs.shader)
                .map_err(|err| CookingError::from_err(source_id.as_str(), err))?;
            if id.extension() != "fs" {
                return Err(CookingError::from_err(
                    source_id.as_str(),
                    AssetError::UnsupportedFormat(id.extension().to_owned()),
                ));
            }
            fs.shader = cookers.cook_shader(id, Naming::hard("shader", "fs")).await?.to_string();
        }

        Ok(CookedPipeline { descriptor })
    }
}
