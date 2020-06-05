use shine_game::assets::{AssetIO, AssetId, Url};
use std::sync::Arc;
use tokio::runtime::Runtime;

mod cache_db;
mod config;
mod cook_gltf;
mod cook_pipeline;
mod cook_shader;
mod cook_texture;
mod cook_world;
mod error;
mod target_db;

pub use self::config::Config;
pub use cache_db::{CacheDB, SourceCacheEntry};
pub use error::CookingError;
pub use target_db::{AssetNaming, Dependency, TargetDB};

#[derive(Clone)]
pub struct Context {
    pub source_io: Arc<AssetIO>,
    pub cache_db: CacheDB,
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

/*async fn find_cook_roots(context: &Context, assets_url: &Vec<Url>) -> Result<Vec<Url>, CookingError> {
    //source
    let assets_str = assets_url.iter().map(|url| url.as_str()).collect::<Vec<_>>();
    log::info!("seeds: {:?}", assets_str);

    // source -> cooked
    let cooked_assets_url = context.cache_db.get_cooked_urls(&assets_str[..]).await?;
    let cooked_assets_str = cooked_assets_url.iter().map(|url| url.as_str()).collect::<Vec<_>>();
    log::info!("cooked_assets: {:?}", cooked_assets_str);

    // cooked dependencies
    let cooked_roots_url = context.target_db.get_affected_roots(&cooked_assets_str[..]).await?;
    let cooked_roots_str = cooked_roots_url.iter().map(|url| url.as_str()).collect::<Vec<_>>();
    log::info!("cooked roots: {:?}", cooked_roots_str);

    // cooked -> source
    let roots_str = context.cache_db.get_source_urls(&cooked_roots_str[..]).await?;
    let roots_url = roots_str
        .iter()
        .map(|url| Url::parse(&url))
        .collect::<Result<Vec<_>, _>>()?;
    log::info!("roots: {:?}", roots_url);

    Ok(roots_url)
}
*/
async fn run(assets: Vec<AssetId>) -> Result<(), CookingError> {
    let config = Config::new().unwrap();

    let context = {
        let source_io = Arc::new(AssetIO::new(config.source_virtual_schemes.clone())?);
        let cache_db = CacheDB::new(&config).await?;
        let target_db = TargetDB::new(&config).await?;
        Context {
            source_io,
            cache_db,
            target_db,
        }
    };

    //let roots = find_cook_roots(&context, &assets).await?;

    for asset_id in &assets {
        let _cooked_dependency = cook(&context, &config.asset_source_base, &asset_id).await?;
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
        //"test_worlds/test1/hello.fs",
        "test_worlds/test1/test.wrld",
        "test_worlds/test2/test.wrld",
        //"test_worlds/test3/test.wrld",
        //"test_worlds/test4/test.wrld",
    ]
    .iter()
    .map(|x| AssetId::new(x).unwrap())
    .collect();

    if let Err(err) = rt.block_on(run(assets)) {
        println!("Cooking failed: {}", err);
    }
}
