use color_eyre::{self, Report};
use shine_game::assets::{self, AssetIO, AssetId, Url, UrlError};
use std::sync::Arc;
use thiserror::Error;
use tokio::runtime::Runtime;

mod config;
//mod cook_frame_graph;
//mod cook_game;
//mod cook_gltf;
//mod cook_pipeline;
mod cook_shader;
mod cook_texture;
mod target_db;

pub use self::config::Config;
pub use target_db::TargetDB;

#[derive(Debug, Error)]
pub enum CookerError {
    #[error(transparent)]
    Url(#[from] assets::UrlError),
    #[error(transparent)]
    Asset(#[from] assets::AssetError),
    #[error(transparent)]
    Cook(#[from] assets::CookingError),

    #[error("Runtime error")]
    Runtime(#[from] tokio::task::JoinError),

    #[error("Serialization error - json")]
    Json(#[from] serde_json::Error),
    #[error("Serialization error - binary")]
    Bincode(#[from] bincode::Error),

    #[error("Database error")]
    SqlDb(#[from] sqlx::Error),
}

/// Asset source id. Actually all of them is relative to some storage base,
/// but within the starage they can be relative or absolute.
#[derive(Clone, Debug)]
pub enum SourceId {
    Relative(String, AssetId),
    Absolute(AssetId),
}

impl SourceId {
    pub fn extension(&self) -> &str {
        match self {
            SourceId::Absolute(id) => id.extension(),
            SourceId::Relative(_, id) => id.extension(),
        }
    }

    pub fn to_asset_id(&self) -> Result<AssetId, UrlError> {
        match self {
            SourceId::Absolute(id) => Ok(id.clone()),
            SourceId::Relative(base, id) => Ok(id.into_absolute(&base)?),
        }
    }

    pub fn to_url(&self, base: &Url) -> Result<Url, UrlError> {
        self.to_asset_id()?.to_url(base)
    }
}

/// Indicates, how to name the cooked assets
pub enum TargetNaming {
    Hard(String, Option<String>),
    Soft(String),
}

/// Define the source -> cooked mapping for asset dependency handling.
/// There are two type of dependency:
/// - hard which requires a recooking of the dependant assets
/// - soft where dependency graph can be cut w.r.t. asset cooking
/// Also note that all ids are replaced by a storage_root relative id
pub struct Dependency {
    source_id: SourceId,
    cooked_url: Url,
    is_soft: bool,
}

impl Dependency {
    pub fn soft(source_id: SourceId, cooked_url: Url) -> Dependency {
        Dependency {
            source_id,
            cooked_url,
            is_soft: true,
        }
    }

    pub fn hard(source_id: SourceId, cooked_url: Url) -> Dependency {
        Dependency {
            source_id,
            cooked_url,
            is_soft: false,
        }
    }

    pub fn is_soft(&self) -> bool {
        self.is_soft
    }

    pub fn is_hard(&self) -> bool {
        !self.is_soft
    }

    pub fn source_id(&self) -> &SourceId {
        &self.source_id
    }

    pub fn cooked_url(&self) -> &Url {
        &self.cooked_url
    }
}

#[derive(Clone)]
pub struct Context {
    // all asset source is located in the root
    pub source_root: Url,
    pub source_io: Arc<AssetIO>,
    pub target_db: TargetDB,
}

async fn cook(context: &Context, source_id: SourceId) -> Result<Dependency, CookerError> {
    let cooked_dependency = match source_id.extension() {
        "vs" | "fs" | "cs" => cook_shader::cook_shader(&context, source_id).await?,
        //"pl" => cook_pipeline::cook_pipeline(&context, &asset_base, &asset_id).await?,
        //"fgr" => cook_frame_graph::cook_frame_graph(&context, &asset_base, &asset_id).await?,
        //"glb" | "gltf" => cook_gltf::cook_gltf(&context, &asset_base, &asset_id).await?,
        "jpg" | "png" => cook_texture::cook_texture(&context, source_id).await?,
        //"game" => cook_game::cook_game(&context, &asset_base, &asset_id).await?,
        e => return Err(assets::AssetError::UnsupportedFormat(e.into()).into()),
    };

    Ok(cooked_dependency)
}

async fn run(assets: Vec<AssetId>) -> Result<(), CookerError> {
    let config = Config::new().unwrap();

    let context = {
        let source_io = Arc::new(AssetIO::new(config.source_virtual_schemes.clone())?);
        let target_db = TargetDB::new(&config).await?;
        Context {
            source_root: config.source_root.clone(),
            source_io,
            target_db,
        }
    };

    let root_assets = context.target_db.get_affected_roots(&assets[..]).await?;
    log::info!("Root assets to cook: {:?}", root_assets);

    for asset_id in &root_assets {
        log::info!("Cooking started for {:?}", asset_id);
        let _cooked_dependency = cook(&context, SourceId::Absolute(asset_id.clone())).await?;
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
        "games/test1/hello.fs",
        "games/test3/checker.png",
        //"games/test1/test.game",
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
