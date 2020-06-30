use crate::{cook_pipeline, cook_texture, AssetId, AssetNaming, Context, CookingError, Dependency};
use shine_game::assets::Url;

async fn find_world_etag(context: &Context, world_url: &Url) -> Result<String, CookingError> {
    Ok(context.source_io.download_etag(&world_url).await?)
}

pub async fn get_world_etag(context: &Context, asset_base: &Url, world_id: &AssetId) -> Result<String, CookingError> {
    let world_url = world_id.to_url(asset_base)?;
    find_world_etag(context, &world_url).await
}

pub async fn cook_world(context: &Context, asset_base: &Url, world_id: &AssetId) -> Result<Dependency, CookingError> {
    let world_url = world_id.to_url(asset_base)?;

    log::info!("[{}] Cooking...", world_url.as_str());
    let source_hash = find_world_etag(context, &world_url).await?;

    log::debug!("[{}] Downloading...", world_url.as_str());
    let data = context.source_io.download_binary(&world_url).await?;
    let mut world = serde_json::from_slice::<WorldData>(&data)?;
    log::trace!("[{}] World:\n{:#?}", world_url.as_str(), world);

    let mut dependnecies = Vec::new();
    let world_base = world_url.to_folder()?;

    log::debug!("[{}] Cooking world content...", world_url.as_str());
    match world {
        WorldData::Test1(ref mut test) => {
            let pipeline_id = AssetId::new(&test.pipeline)?.to_absolute_id(asset_base, &world_base)?;
            let pipeline_dependency = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_id).await?;
            test.pipeline = pipeline_dependency.url().as_str().to_owned();
            dependnecies.push(pipeline_dependency);
        }
        WorldData::Test2(ref mut test) => {
            let pipeline_id = AssetId::new(&test.pipeline)?.to_absolute_id(asset_base, &world_base)?;
            let pipeline_dependency = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_id).await?;
            test.pipeline = pipeline_dependency.url().as_str().to_owned();
            dependnecies.push(pipeline_dependency);
        }
        WorldData::Test3(ref mut test) => {
            let pipeline_id = AssetId::new(&test.pipeline)?.to_absolute_id(asset_base, &world_base)?;
            let pipeline_dependency = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_id).await?;
            test.pipeline = pipeline_dependency.url().as_str().to_owned();
            dependnecies.push(pipeline_dependency);

            let texture_id = AssetId::new(&test.texture)?.to_absolute_id(asset_base, &world_base)?;
            let texture_dependency = cook_texture::cook_texture(context, asset_base, &texture_id).await?;
            test.texture = texture_dependency.url().as_str().to_owned();
            dependnecies.push(texture_dependency);
        }
        WorldData::Test4(ref mut test) => {
            let pipeline_id = AssetId::new(&test.pipeline)?.to_absolute_id(asset_base, &world_base)?;
            let pipeline_dependency = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_id).await?;
            test.pipeline = pipeline_dependency.url().as_str().to_owned();
            dependnecies.push(pipeline_dependency);

            let texture_id = AssetId::new(&test.texture)?.to_absolute_id(asset_base, &world_base)?;
            let texture_dependency = cook_texture::cook_texture(context, asset_base, &texture_id).await?;
            test.texture = texture_dependency.url().as_str().to_owned();
            dependnecies.push(texture_dependency);
        }
    }
    log::trace!("[{}] Cooked world:\n{:#?}", world_url.as_str(), world);

    log::debug!("[{}] Uploading...", world_url.as_str());
    let cooked_world = bincode::serialize(&world)?;
    Ok(context
        .target_db
        .upload_cooked_binary(
            world_id.clone(),
            world_url,
            AssetNaming::SoftScheme("world".to_owned()),
            &cooked_world,
            source_hash,
            dependnecies,
        )
        .await?)
}
