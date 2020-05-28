use crate::{cook_pipeline, cook_texture, Context, CookingError};
use shine_game::assets::{AssetError, Url};
use shine_game::world::World;

pub async fn cook_world(
    context: &Context,
    source_base: &Url,
    target_base: &Url,
    world_url: &Url,
) -> Result<String, CookingError> {
    log::info!("[{}] Cooking...", world_url.as_str());

    log::debug!("[{}] Downloading...", world_url.as_str());
    let data = context.assetio.download_binary(&world_url).await?;
    let mut world = serde_json::from_slice::<World>(&data)?;
    log::trace!("[{}] World:\n{:#?}", world_url.as_str(), world);

    log::debug!("[{}] Cooking world content...", world_url.as_str());
    match world {
        World::Test1(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(source_base, world_url, &test.pipeline)?;
            test.pipeline = cook_pipeline::cook_pipeline(context, source_base, target_base, &pipeline_url).await?;
        }
        World::Test2(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(source_base, world_url, &test.pipeline)?;
            test.pipeline = cook_pipeline::cook_pipeline(context, source_base, target_base, &pipeline_url).await?;
        }
        World::Test3(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(source_base, world_url, &test.pipeline)?;
            let texture_url = Url::from_base_or_current(source_base, world_url, &test.texture)?;

            test.pipeline = cook_pipeline::cook_pipeline(context, source_base, target_base, &pipeline_url).await?;
            test.texture = cook_texture::cook_texture(context, source_base, target_base, &texture_url).await?;
        }
        World::Test4(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(source_base, world_url, &test.pipeline)?;
            let texture_url = Url::from_base_or_current(source_base, world_url, &test.texture)?;

            test.pipeline = cook_pipeline::cook_pipeline(context, source_base, target_base, &pipeline_url).await?;
            test.texture = cook_texture::cook_texture(context, source_base, target_base, &texture_url).await?;
        }
    }
    log::trace!("[{}] Cooked world:\n{:#?}", world_url.as_str(), world);

    log::error!("prefix{:?}", world_url.relative_path(&source_base));

    let target_url = target_base.join(world_url.relative_path(&source_base).ok_or(
        AssetError::UnsupportedFormat("Falied to create path for target".to_owned()),
    )?)?;
    log::debug!("[{}] Uploading...", world_url.as_str());
    let cooked_world = bincode::serialize(&world)?;
    context.assetio.upload_binary(&target_url, &cooked_world).await?;

    Ok(target_url.as_str().to_owned())
}
