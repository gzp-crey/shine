use crate::{cook_pipeline, cook_texture, AssetNaming, Context, CookingError, TargetDependency};
use shine_game::assets::Url;
use shine_game::world::World;

pub async fn get_world_etag(context: &Context, world_url: &Url) -> Result<String, CookingError> {
    Ok(context.source_io.download_etag(&world_url).await?)
}

pub async fn cook_world(
    context: &Context,
    asset_base: &Url,
    world_url: &Url,
) -> Result<TargetDependency, CookingError> {
    log::info!("[{}] Cooking...", world_url.as_str());
    let source_hash = get_world_etag(context, &world_url).await?;

    log::debug!("[{}] Downloading...", world_url.as_str());
    let data = context.source_io.download_binary(&world_url).await?;
    let mut world = serde_json::from_slice::<World>(&data)?;
    log::trace!("[{}] World:\n{:#?}", world_url.as_str(), world);

    let mut dependnecies = Vec::new();

    log::debug!("[{}] Cooking world content...", world_url.as_str());
    match world {
        World::Test1(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(asset_base, world_url, &test.pipeline)?;
            let pipeline_id = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_url).await?;
            test.pipeline = pipeline_id.url().to_owned();
            dependnecies.push(pipeline_id);
        }
        World::Test2(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(asset_base, world_url, &test.pipeline)?;
            let pipeline_id = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_url).await?;
            test.pipeline = pipeline_id.url().to_owned();
            dependnecies.push(pipeline_id);
        }
        World::Test3(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(asset_base, world_url, &test.pipeline)?;
            let pipeline_id = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_url).await?;
            test.pipeline = pipeline_id.url().to_owned();
            dependnecies.push(pipeline_id);

            let texture_url = Url::from_base_or_current(asset_base, world_url, &test.texture)?;
            let texture_id = cook_texture::cook_texture(context, asset_base, &texture_url).await?;
            test.texture = texture_id.url().to_owned();
            dependnecies.push(texture_id);
        }
        World::Test4(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(asset_base, world_url, &test.pipeline)?;
            let pipeline_id = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_url).await?;
            test.pipeline = pipeline_id.url().to_owned();
            dependnecies.push(pipeline_id);

            let texture_url = Url::from_base_or_current(asset_base, world_url, &test.texture)?;
            let texture_id = cook_texture::cook_texture(context, asset_base, &texture_url).await?;
            test.texture = texture_id.url().to_owned();
            dependnecies.push(texture_id);
        }
    }
    log::trace!("[{}] Cooked world:\n{:#?}", world_url.as_str(), world);

    log::debug!("[{}] Uploading...", world_url.as_str());
    let cooked_world = bincode::serialize(&world)?;
    let cooked_dependency = context
        .target_db
        .upload_cooked_binary(
            &asset_base,
            &world_url,
            AssetNaming::SoftScheme("world".to_owned()),
            &cooked_world,
            dependnecies,
        )
        .await?;
    context
        .cache_db
        .set_info(world_url.as_str(), &source_hash, cooked_dependency.url())
        .await?;
    Ok(cooked_dependency)
}
