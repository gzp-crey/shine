use crate::{cook_pipeline, cook_texture, Context, CookingError};
use shine_game::assets::{AssetNaming, Url};
use shine_game::world::World;

pub async fn cook_world(
    context: &Context,
    asset_base: &Url,
    world_url: &Url,
) -> Result<Url, CookingError> {
    log::info!("[{}] Cooking...", world_url.as_str());

    log::debug!("[{}] Downloading...", world_url.as_str());
    let data = context.source_io.download_binary(&world_url).await?;
    let mut world = serde_json::from_slice::<World>(&data)?;
    log::trace!("[{}] World:\n{:#?}", world_url.as_str(), world);

    log::debug!("[{}] Cooking world content...", world_url.as_str());
    match world {
        World::Test1(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(asset_base, world_url, &test.pipeline)?;
            test.pipeline = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_url)
                .await?
                .as_str()
                .to_owned();
        }
        World::Test2(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(asset_base, world_url, &test.pipeline)?;
            test.pipeline = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_url)
                .await?
                .as_str()
                .to_owned();
        }
        World::Test3(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(asset_base, world_url, &test.pipeline)?;
            let texture_url = Url::from_base_or_current(asset_base, world_url, &test.texture)?;

            test.pipeline = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_url)
                .await?
                .as_str()
                .to_owned();
            test.texture = cook_texture::cook_texture(context, asset_base, &texture_url)
                .await?
                .as_str()
                .to_owned();
        }
        World::Test4(ref mut test) => {
            let pipeline_url = Url::from_base_or_current(asset_base, world_url, &test.pipeline)?;
            let texture_url = Url::from_base_or_current(asset_base, world_url, &test.texture)?;

            test.pipeline = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_url)
                .await?
                .as_str()
                .to_owned();
            test.texture = cook_texture::cook_texture(context, asset_base, &texture_url)
                .await?
                .as_str()
                .to_owned();
        }
    }
    log::trace!("[{}] Cooked world:\n{:#?}", world_url.as_str(), world);

    log::debug!("[{}] Uploading...", world_url.as_str());
    let cooked_world = bincode::serialize(&world)?;
    Ok(context
        .target_io
        .upload_cooked_binary(
            &asset_base,
            &world_url,
            AssetNaming::VirtualScheme("world".to_owned()),
            &cooked_world,
        )
        .await?)
}
