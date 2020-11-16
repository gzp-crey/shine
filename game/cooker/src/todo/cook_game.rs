use crate::{cook_frame_graph, cook_pipeline, cook_texture, AssetNaming, Context, CookingError, Dependency};
use shine_game::assets::{AssetId, Url};
use shine_game::game::Game;

async fn find_game_etag(context: &Context, game_url: &Url) -> Result<String, CookingError> {
    Ok(context.source_io.download_etag(&game_url).await?)
}

pub async fn get_game_etag(context: &Context, asset_base: &Url, game_id: &AssetId) -> Result<String, CookingError> {
    let game_url = game_id.to_url(asset_base)?;
    find_game_etag(context, &game_url).await
}

pub async fn cook_game(context: &Context, asset_base: &Url, game_id: &AssetId) -> Result<Dependency, CookingError> {
    let game_url = game_id.to_url(asset_base)?;

    log::info!("[{}] Cooking...", game_url.as_str());
    let source_hash = find_game_etag(context, &game_url).await?;

    log::debug!("[{}] Downloading...", game_url.as_str());
    let data = context.source_io.download_binary(&game_url).await?;
    let mut game = serde_json::from_slice::<Game>(&data)?;
    log::trace!("[{}] Game:\n{:#?}", game_url.as_str(), game);

    let mut dependnecies = Vec::new();
    let game_base = game_url.to_folder()?;
    log::debug!("[{}] Cooking game content...", game_url.as_str());

    match game {
        Game::Test1(ref mut test) => {
            let pipeline_id = AssetId::new(&test.pipeline)?.to_absolute_id(asset_base, &game_base)?;
            let pipeline_dependency = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_id).await?;
            test.pipeline = pipeline_dependency.url().as_str().to_owned();
            dependnecies.push(pipeline_dependency);
        } /*Game::Test2(ref mut test) => {
              let pipeline_id = AssetId::new(&test.pipeline)?.to_absolute_id(asset_base, &game_base)?;
              let pipeline_dependency = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_id).await?;
              test.pipeline = pipeline_dependency.url().as_str().to_owned();
              dependnecies.push(pipeline_dependency);
          }
          Game::Test3(ref mut test) => {
              let pipeline_id = AssetId::new(&test.pipeline)?.to_absolute_id(asset_base, &game_base)?;
              let pipeline_dependency = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_id).await?;
              test.pipeline = pipeline_dependency.url().as_str().to_owned();
              dependnecies.push(pipeline_dependency);

              let texture_id = AssetId::new(&test.texture)?.to_absolute_id(asset_base, &game_base)?;
              let texture_dependency = cook_texture::cook_texture(context, asset_base, &texture_id).await?;
              test.texture = texture_dependency.url().as_str().to_owned();
              dependnecies.push(texture_dependency);
          }
          Game::Test4(ref mut test) => {
              let pipeline_id = AssetId::new(&test.pipeline)?.to_absolute_id(asset_base, &game_base)?;
              let pipeline_dependency = cook_pipeline::cook_pipeline(context, asset_base, &pipeline_id).await?;
              test.pipeline = pipeline_dependency.url().as_str().to_owned();
              dependnecies.push(pipeline_dependency);

              let texture_id = AssetId::new(&test.texture)?.to_absolute_id(asset_base, &game_base)?;
              let texture_dependency = cook_texture::cook_texture(context, asset_base, &texture_id).await?;
              test.texture = texture_dependency.url().as_str().to_owned();
              dependnecies.push(texture_dependency);
          }
          Game::Test5(ref mut test) => {
              let frame_graph_id = AssetId::new(&test.frame_graph)?.to_absolute_id(asset_base, &game_base)?;
              let frame_graph_dependency =
                  cook_frame_graph::cook_frame_graph(context, asset_base, &frame_graph_id).await?;
              test.frame_graph = frame_graph_dependency.url().as_str().to_owned();
              dependnecies.push(frame_graph_dependency);

              let scene_pipeline_id = AssetId::new(&test.scene_pipeline)?.to_absolute_id(asset_base, &game_base)?;
              let scene_pipeline_dependency =
                  cook_pipeline::cook_pipeline(context, asset_base, &scene_pipeline_id).await?;
              test.scene_pipeline = scene_pipeline_dependency.url().as_str().to_owned();
              dependnecies.push(scene_pipeline_dependency);

              let present_pipeline_id = AssetId::new(&test.present_pipeline)?.to_absolute_id(asset_base, &game_base)?;
              let present_pipeline_dependency =
                  cook_pipeline::cook_pipeline(context, asset_base, &present_pipeline_id).await?;
              test.present_pipeline = present_pipeline_dependency.url().as_str().to_owned();
              dependnecies.push(present_pipeline_dependency);
          }*/
    }
    log::trace!("[{}] Cooked game:\n{:#?}", game_url.as_str(), game);

    log::debug!("[{}] Uploading...", game_url.as_str());
    let cooked_game = bincode::serialize(&game)?;
    Ok(context
        .target_db
        .upload_cooked_binary(
            game_id.clone(),
            game_url,
            AssetNaming::SoftScheme("game".to_owned()),
            &cooked_game,
            source_hash,
            dependnecies,
        )
        .await?)
}
