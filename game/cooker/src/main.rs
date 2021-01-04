use color_eyre::{self, Report};
use shine_game::assets::{
    cooker::{CookingError, ModelCooker, Naming, PipelineCooker, ShaderCooker, TextureCooker},
    AssetError, AssetIO, AssetId, Url, UrlError,
};
use thiserror::Error;
use tokio::runtime::Runtime;

mod config;
//mod cook_frame_graph;
mod cook_game;
mod cook_model;
mod cook_pipeline;
mod cook_shader;
mod cook_texture;
mod target_db;

pub use self::config::Config;
pub use target_db::TargetDB;

#[derive(Debug, Error)]
pub enum CookerError {
    #[error(transparent)]
    Url(#[from] UrlError),

    #[error(transparent)]
    Asset(#[from] AssetError),

    #[error(transparent)]
    Cook(#[from] CookingError),

    #[error(transparent)]
    Config(#[from] ::config::ConfigError),

    #[error("Runtime error")]
    Runtime(#[from] tokio::task::JoinError),

    #[error("Database error")]
    SqlDb(#[from] sqlx::Error),
}

#[derive(Clone)]
pub struct Context {
    // all asset source is located in this root
    pub source_root: Url,
    pub source_io: AssetIO,
    pub target_io: TargetDB,
}

impl Context {
    pub fn create_scope(&self, asset_scope: AssetId) -> Context {
        Context {
            source_root: self.source_root.clone(),
            source_io: self.source_io.clone(),
            target_io: self.target_io.create_scope(asset_scope),
        }
    }
}

async fn cook(context: &Context, source_id: AssetId) -> Result<Url, CookerError> {
    let ext = source_id.extension();
    let cooked_dependency = match ext {
        "vs" | "fs" | "cs" => {
            context
                .cook_shader(source_id.clone(), Naming::soft("shader", ext))
                .await?
        }
        "pl" => {
            context
                .cook_pipeline(source_id.clone(), Naming::soft("pipeline", "pl"))
                .await?
        }
        "glb" | "gltf" => {
            context
                .cook_model(source_id.clone(), Naming::soft("model", "md"))
                .await?
        }
        "jpg" | "png" => {
            context
                .cook_texture(source_id.clone(), Naming::soft("texture", "tx"))
                .await?
        }
        "game" => context.cook_game(source_id.clone()).await?,
        e => return Err(AssetError::UnsupportedFormat(e.into()).into()),
    };

    Ok(cooked_dependency)
}

async fn run(assets: Vec<AssetId>) -> Result<(), CookerError> {
    let config = Config::new().unwrap();

    let context = {
        let source_io = AssetIO::new(config.source_virtual_schemes.clone())?;
        let target_io = TargetDB::new(&config).await?;
        Context {
            source_root: config.source_root.clone(),
            source_io,
            target_io,
        }
    };

    //let root_assets = context.target_io.get_affected_roots(&assets[..]).await?;
    //log::info!("Root assets to cook: {:?}", root_assets);

    for asset_id in &assets {
        log::info!("Cooking started for {}", asset_id);
        let _cooked_dependency = cook(&context, asset_id.clone()).await?;
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
        //"games/test/test1/hello.fs",
        //"games/test/test3/checker.png",
        //"models/SimpleMeshes.gltf",
        //"models/VertexColorTest.glb",
        "games/test/test1.game",
        //"games/test/test2.game",
        //"games/test/test3.game",
        //"games/test/test4.game",
        //"games/test/test5.game",
    ]
    .iter()
    .map(|x| AssetId::new(x))
    .collect::<Result<Vec<_>, _>>()?;

    rt.block_on(run(assets))?;
    Ok(())
}
