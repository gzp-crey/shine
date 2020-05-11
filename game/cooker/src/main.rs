use shine_game::utils::url::Url;
use tokio::runtime::Runtime;

mod config;
mod content_hash;
mod cook_gltf;
mod cook_pipeline;
mod cook_shader;
mod cook_texture;

async fn run(asset: String) {
    let config = config::Config::new().unwrap();
    let asset_source_base = Url::parse(&config.asset_source_base).unwrap();
    let asset_target_base = Url::parse(&config.asset_target_base).unwrap();

    let asset_url = asset_source_base.join(&asset).unwrap();

    let result = match asset_url.extension() {
        "vs" | "fs" | "cs" => cook_shader::cook_shader(&asset_source_base, &asset_target_base, &asset_url).await,
        "pl" => cook_pipeline::cook_pipeline(&asset_source_base, &asset_target_base, &asset_url).await,
        "glb" | "gltf" => cook_gltf::cook_gltf(&asset_source_base, &asset_target_base, &asset_url).await,
        "jpg" | "png" => cook_texture::cook_texture(&asset_source_base, &asset_target_base, &asset_url).await,
        e => Err(format!("Unknown asset type: {}", e)),
    };

    match result {
        Ok(hashed_id) => log::info!("Cooking of [{}] done: [{}]", asset, hashed_id),
        Err(err) => log::error!("Cooking of [{}] failed: {}", asset, err),
    };
}

fn main() {
    shine_game::render::foo();

    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .filter_module("shine-ecs", log::LevelFilter::Debug)
        .filter_module("shine-game", log::LevelFilter::Trace)
        .try_init();
    let mut rt = Runtime::new().unwrap();

    //let asset = "pipelines/hello/hello.vs".to_owned();
    //let asset = "pipelines/hello/hello.pl".to_owned();
    //let asset = "pipelines/hello2/hello.pl".to_owned();
    let asset = "tex/checker.png".to_owned();
    rt.block_on(run(asset));
}

// todo: sqlite local DB
//  content_source -> source hash -> cooked hash
// cook:
//  if source_hash changes {
//     run cooker
//     if remote hash changes {
//         update blob
//         update cooked hash
//     }
//     update source hash
// }
//
