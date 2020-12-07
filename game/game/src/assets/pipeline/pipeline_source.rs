#[cfg(feature = "cook")]
use crate::assets::{io::HashableContent, AssetError, AssetIO, CookedPipeline, CookingError, PipelineDescriptor, Url};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PipelineSource {
    pub source_url: Url,
    pub descriptor: PipelineDescriptor,
}

impl PipelineSource {
    pub async fn load(io: &AssetIO, source_url: &Url) -> Result<(Self, String), AssetError> {
        log::debug!("[{}] Downloading...", source_url.as_str());
        let data = io.download_binary(&source_url).await?;

        let pipeline = serde_json::from_slice::<PipelineDescriptor>(&data)
            .map_err(|err| AssetError::load_failed(source_url.as_str(), err))?;
        log::trace!("[{}] Pipeline:\n{:#?}", source_url.as_str(), pipeline);

        let source = PipelineSource {
            source_url: source_url.clone(),
            descriptor: pipeline,
        };
        let source_hash = data.content_hash();
        Ok((source, source_hash))
    }

    pub async fn cook(self) -> Result<CookedPipeline, CookingError> {
        log::debug!("[{}] Compiling...", self.source_url.as_str());

        let PipelineSource {
            source_url,
            /*mut*/ descriptor,
        } = self;

        // perform some consistency check
        for scope in [
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
        }

        // cook dependencies
        log::debug!("[{}] Checking vertex shader dependency...", source_url.as_str());
        //let vertex_shader_id = AssetId::new(&pipeline.vertex_stage.shader)?.to_absolute_id(dependency_base, &pipeline_base)?;
        //let vertex_shader_dependency = cook_shader::cook_shader(context, asset_base, &vertex_shader_id).await?;
        //pipeline.vertex_stage.shader = vertex_shader_dependency.url().as_str().to_owned();

        log::debug!("[{}] Checking fragment shader dependency...", source_url.as_str());
        //let fragment_shader_id = AssetId::new(&pipeline.fragment_stage.shader)?.to_absolute_id(asset_base, &pipeline_base)?;
        //let fragment_shader_dependency = cook_shader::cook_shader(context, asset_base, &fragment_shader_id).await?;
        //pipeline.fragment_stage.shader = fragment_shader_dependency.url().as_str().to_owned();

        Ok(CookedPipeline { descriptor })
    }
}
