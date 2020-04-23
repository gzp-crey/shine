use tokio::runtime::Runtime;
use shine_game::utils::url::Url;

mod config;
mod content_hash;
mod cook_shader;
mod cook_pipeline;

async fn run() {
    let config = config::Config::new().unwrap();
    let asset_source_base = Url::parse(&config.asset_source_base).unwrap();
    let asset_target_base = Url::parse(&config.asset_target_base).unwrap();

    let pipeline = "pipeline/flat.pl";
    match cook_pipeline::cook(&asset_source_base, &asset_target_base, pipeline).await {
        Ok((f, t)) => log::info!("Cooking pipeline done: [{}] -> [{}]", f, t),
        Err(err) => log::error!("Cooking pipeline {} failed: {}", pipeline, err),
    }
}

fn main() {
    //shine_game::render::foo();
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .filter_module("shine-ecs", log::LevelFilter::Debug)
        .filter_module("shine-game", log::LevelFilter::Trace)
        .try_init();
    let mut rt = Runtime::new().unwrap();

    rt.block_on(run());
}
