use shine_game::assets::{AssetIO, AssetId, Url};
use std::sync::Arc;
use tokio::runtime::Runtime;

mod config;
mod cook_gltf;
mod cook_pipeline;
mod cook_shader;
mod cook_texture;
mod cook_world;
mod error;
mod target_db;

pub use self::config::Config;
pub use error::CookingError;
pub use target_db::{AssetNaming, Dependency, TargetDB};

#[derive(Clone)]
pub struct Context {
    pub source_io: Arc<AssetIO>,
    pub target_db: TargetDB,
}

async fn cook(context: &Context, asset_base: &Url, asset_id: &AssetId) -> Result<Dependency, CookingError> {
    let cooked_dependency = match asset_id.extension() {
        "vs" | "fs" | "cs" => cook_shader::cook_shader(&context, &asset_base, &asset_id).await?,
        "pl" => cook_pipeline::cook_pipeline(&context, &asset_base, &asset_id).await?,
        "glb" | "gltf" => cook_gltf::cook_gltf(&context, &asset_base, &asset_id).await?,
        "jpg" | "png" => cook_texture::cook_texture(&context, &asset_base, &asset_id).await?,
        "wrld" => cook_world::cook_world(&context, &asset_base, &asset_id).await?,
        e => return Err(CookingError::Other(format!("Unknown asset type: {}", e))),
    };

    Ok(cooked_dependency)
}

async fn run(assets: Vec<AssetId>) -> Result<(), CookingError> {
    let config = Config::new().unwrap();

    let context = {
        let source_io = Arc::new(AssetIO::new(config.source_virtual_schemes.clone())?);
        let target_db = TargetDB::new(&config).await?;
        Context { source_io, target_db }
    };

    let roots = context.target_db.get_affected_roots(&assets[..]).await?;
    log::info!("Roots to cook: {:?}", roots);

    for asset_id in &roots {
        log::info!("Cooking started for {:?}", asset_id);
        let _cooked_dependency = cook(&context, &config.asset_source_base, &asset_id).await?;
        log::info!("Cooking completed for {:?}", asset_id);
    }

    Ok(())
}

fn main() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("shine_cooker", log::LevelFilter::Trace)
        .filter_module("shine_ecs", log::LevelFilter::Debug)
        .filter_module("shine_game", log::LevelFilter::Trace)
        .filter_module("sqlx_core::postgres::executor", log::LevelFilter::Trace)
        .try_init();
    let mut rt = Runtime::new().unwrap();

    let assets = [
        "test_worlds/test3/hello.pl",
        "test_worlds/test1/test.wrld",
        "test_worlds/test2/test.wrld",
        "test_worlds/test3/test.wrld",
        "test_worlds/test4/test.wrld",
    ]
    .iter()
    .map(|x| AssetId::new(x).unwrap())
    .collect();

    if let Err(err) = rt.block_on(run(assets)) {
        println!("Cooking failed: {}", err);
    }
}
