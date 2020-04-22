use shaderc;
use shine_game::utils::{assets, url::Url};
use tokio::runtime::Runtime;

mod config;
mod content_hash;

pub async fn cook_shader(sourc_base: &Url, target_base: &Url, source_id: &str) -> Result<(String,String), String> {
    let source_url = sourc_base
        .join(source_id)
        .map_err(|err| format!("Invalid source url: {:?}", err))?;
    log::trace!("Downloading shader source from {}", source_url.as_str());
    let shader_source = assets::download_string(&source_url)
        .await
        .map_err(|err| format!("Failed to get source conent: {:?}", err))?;

    let ext = source_url.extension();
    let ty = match ext {
        "vs" => shaderc::ShaderKind::Vertex,
        "fs" => shaderc::ShaderKind::Fragment,
        "cs" => shaderc::ShaderKind::Compute,
        _ => return Err(format!("Unknown shader type: {}", ext)),
    };
    log::trace!("Compiling {:?} shader", ty);
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    let compiled_artifact = compiler
        .compile_into_spirv(&shader_source, ty, source_url.as_str(), "main", Some(&options))
        .map_err(|err| format!("Shader compilation failed: {:?}", err))?;

    let hash = content_hash::sha256_bytes(shader_source.as_bytes());
    let target_id = format!("{}.{}_spv", hash, ext);
    let target_url = target_base
        .join(&target_id)
        .map_err(|err| format!("Invalid target url: {:?}", err))?;
    log::trace!("Uploading shader binary as: {}", target_url.as_str());
    assets::upload_binary(&target_url, compiled_artifact.as_binary_u8())
        .await
        .map_err(|err| format!("Failed to upload {}: {:?}", target_url.as_str(), err))?;

    Ok((source_id.to_owned(), target_id))
}

async fn run() {
    let config = config::Config::new().unwrap();
    let asset_source_base = Url::parse(&config.asset_source_base).unwrap();
    let asset_target_base = Url::parse(&config.asset_target_base).unwrap();

    let shader = "pipeline/hello.vs";
    match cook_shader(&asset_source_base, &asset_target_base, shader).await {
        Ok((f,t)) => log::info!("Cooking shader done: [{}] -> [{}]", f, t),
        Err(err) => log::error!("Cookinf shader {} failed: {}", shader, err),
    }
}

fn main() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .filter_module("shine-ecs", log::LevelFilter::Debug)
        .filter_module("shine-game", log::LevelFilter::Trace)
        .try_init();
    let mut rt = Runtime::new().unwrap();

    rt.block_on(run());
}
