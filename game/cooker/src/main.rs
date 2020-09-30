use color_eyre::{self, Report};
use shine_game::assets::{AssetError, AssetIO, AssetId, Url};
use std::sync::Arc;
use tokio::runtime::Runtime;

mod config;
mod cook_frame_graph;
mod cook_game;
mod cook_gltf;
mod cook_pipeline;
mod cook_shader;
mod cook_texture;
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
        "fgr" => cook_frame_graph::cook_frame_graph(&context, &asset_base, &asset_id).await?,
        "glb" | "gltf" => cook_gltf::cook_gltf(&context, &asset_base, &asset_id).await?,
        "jpg" | "png" => cook_texture::cook_texture(&context, &asset_base, &asset_id).await?,
        "game" => cook_game::cook_game(&context, &asset_base, &asset_id).await?,
        e => return Err(AssetError::UnsupportedFormat(e.into()).into()),
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

fn main() -> Result<(), Report> {
    color_eyre::install()?;
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("shine_cooker", log::LevelFilter::Trace)
        .filter_module("shine_ecs", log::LevelFilter::Debug)
        .filter_module("shine_game", log::LevelFilter::Trace)
        .filter_module("sqlx_core::postgres::executor", log::LevelFilter::Trace)
        .try_init();
    let mut rt = Runtime::new()?;

    let assets = [
        "games/test1/test.game",
        //"games/test2/test.game",
        //"games/test3/test.game",
        //"games/test4/test.game",
        //"games/test5/test.wrld",
    ]
    .iter()
    .map(|x| AssetId::new(x))
    .collect::<Result<Vec<_>, _>>()?;

    rt.block_on(run(assets))?;
    Ok(())
}
