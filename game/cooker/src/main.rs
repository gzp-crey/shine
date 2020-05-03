use shine_game::utils::url::Url;
use tokio::runtime::Runtime;

mod config;
mod content_hash;
mod cook_pipeline;
mod cook_shader;

use cook_pipeline::cook_pipeline;

async fn run() {
    let config = config::Config::new().unwrap();
    let asset_source_base = Url::parse(&config.asset_source_base).unwrap();
    let asset_target_base = Url::parse(&config.asset_target_base).unwrap();

    let pipeline = "pipelines/hello/hello.pl";
    let pipeline_url = asset_source_base.join(pipeline).unwrap();
    match cook_pipeline(&asset_source_base, &asset_target_base, &pipeline_url).await {
        Ok(t) => log::info!("Cooking pipeline done: [{}] -> [{}]", pipeline, t),
        Err(err) => log::error!("Cooking pipeline {} failed: {}", pipeline, err),
    }
}

fn main() {
    shine_game::render::foo();

    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .filter_module("shine-ecs", log::LevelFilter::Debug)
        .filter_module("shine-game", log::LevelFilter::Trace)
        .try_init();
    let mut rt = Runtime::new().unwrap();

    rt.block_on(run());
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
