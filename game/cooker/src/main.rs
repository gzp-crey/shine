use shine_game::assets::{AssetIO, Url};
use std::sync::Arc;
use tokio::runtime::Runtime;

mod config;
mod cook_gltf;
mod cook_pipeline;
mod cook_shader;
mod cook_texture;
mod cook_world;
mod error;
//mod local_db;

pub use self::config::Config;
pub use error::CookingError;
//pub use local_db::{CacheState, CompleteCache, IncompleteCache, LocalDB};

#[derive(Clone)]
pub struct Context {
    pub assetio: Arc<AssetIO>,
    //pub local_db: LocalDB,
}

async fn cook(
    context: &Context,
    source_base: &Url,
    target_base: &Url,
    asset_url: &Url,
) -> Result<String, CookingError> {
    let cooked_id = match asset_url.extension() {
        "vs" | "fs" | "cs" => cook_shader::cook_shader(&context, &source_base, &target_base, &asset_url).await?,
        "pl" => cook_pipeline::cook_pipeline(&context, &source_base, &target_base, &asset_url).await?,
        "glb" | "gltf" => cook_gltf::cook_gltf(&context, &source_base, &target_base, &asset_url).await?,
        "jpg" | "png" => cook_texture::cook_texture(&context, &source_base, &target_base, &asset_url).await?,
        "wrld" => cook_world::cook_world(&context, &source_base, &target_base, &asset_url).await?,
        e => return Err(CookingError::Other(format!("Unknown asset type: {}", e))),
    };

    Ok(cooked_id)
}

async fn run(assets: Vec<String>) -> Result<(), CookingError> {
    let config = Config::new().unwrap();
    let asset_source_base = Url::parse(&config.asset_source_base)?;
    let asset_target_base = Url::parse(&config.asset_target_base)?;

    let context = {
        let assetio = Arc::new(AssetIO::new()?);
        //let local_db = LocalDB::new(&config).await?;
        Context { assetio }
    };

    for asset in &assets {
        let asset_url = asset_source_base.join(&asset)?;
        let _ = cook(&context, &asset_source_base, &asset_target_base, &asset_url).await?;
    }

    Ok(())
}

fn main() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("shine_cooker", log::LevelFilter::Trace)
        .filter_module("shine_ecs", log::LevelFilter::Debug)
        .filter_module("shine_game", log::LevelFilter::Trace)
        .try_init();
    let mut rt = Runtime::new().unwrap();

    let assets: Vec<_> = [
        "test_worlds/test1/test.wrld",
        "test_worlds/test2/test.wrld",
        "test_worlds/test3/test.wrld",
        "test_worlds/test4/test.wrld",
    ]
    .iter()
    .map(|&x| x.to_owned())
    .collect();

    if let Err(err) = rt.block_on(run(assets)) {
        println!("Cooking failed: {}", err);
    }
}

// local DB:
// local relative uri, local source hash, cooked_uri
// cook: if source hash != stored local source hash {
//           cook and update remote db
//           update local
//           return new cooked_uri
//       }
//       else {
//           return cooked_uri from db
//       }
//
// remote db:
//   uri -> dependency uri
