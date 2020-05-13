use shine_game::utils::url::Url;
use std::{error, fmt};
use tokio::runtime::Runtime;

mod config;
mod cook_gltf;
mod cook_pipeline;
mod cook_shader;
mod cook_texture;

#[derive(Debug)]
pub enum CookingError {
    Gltf(cook_gltf::Error),
    Shader(cook_shader::Error),
    Pipeline(cook_pipeline::Error),
    Texture(cook_texture::Error),
    Other(String),
}

impl fmt::Display for CookingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CookingError::Gltf(ref err) => write!(f, "Failed to cook gltf: {}", err),
            CookingError::Shader(ref err) => write!(f, "Failed to cook shader: {}", err),
            CookingError::Pipeline(ref err) => write!(f, "Failed to cook pipeline: {}", err),
            CookingError::Texture(ref err) => write!(f, "Failed to cook texture: {}", err),
            CookingError::Other(ref err) => write!(f, "Cooking failed: {}", err),
        }
    }
}

impl error::Error for CookingError {}

impl From<cook_gltf::Error> for CookingError {
    fn from(err: cook_gltf::Error) -> CookingError {
        CookingError::Gltf(err)
    }
}

impl From<cook_shader::Error> for CookingError {
    fn from(err: cook_shader::Error) -> CookingError {
        CookingError::Shader(err)
    }
}

impl From<cook_pipeline::Error> for CookingError {
    fn from(err: cook_pipeline::Error) -> CookingError {
        CookingError::Pipeline(err)
    }
}

impl From<cook_texture::Error> for CookingError {
    fn from(err: cook_texture::Error) -> CookingError {
        CookingError::Texture(err)
    }
}

async fn cook(source_base: &Url, target_base: &Url, url: &Url) -> Result<String, CookingError> {
    let hashed_file = match url.extension() {
        "vs" | "fs" | "cs" => cook_shader::cook_shader(&source_base, &target_base, &url).await?,
        "pl" => cook_pipeline::cook_pipeline(&source_base, &target_base, &url).await?,
        "glb" | "gltf" => cook_gltf::cook_gltf(&source_base, &target_base, &url).await?,
        "jpg" | "png" => cook_texture::cook_texture(&source_base, &target_base, &url).await?,
        e => return Err(CookingError::Other(format!("Unknown asset type: {}", e))),
    };

    Ok(hashed_file)
}

async fn run(asset: String) {
    let config = config::Config::new().unwrap();
    let asset_source_base = Url::parse(&config.asset_source_base).unwrap();
    let asset_target_base = Url::parse(&config.asset_target_base).unwrap();

    let asset_url = asset_source_base.join(&asset).unwrap();

    match cook(&asset_source_base, &asset_target_base, &asset_url).await {
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

    //let asset = "models/VertexColorTest.glb".to_owned();
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
