use shaderc;
use shine_game::utils::{assets, url::Url};
use tokio::runtime::Runtime;

mod config;
mod content_hash;

async fn cook_shader(sourc_base: &Url, target_base: &Url, shader: &str) -> Result<(), String> {
    let source_url = sourc_base
        .join(shader)
        .map_err(|err| format!("Invalid source url: {:?}", err))?;
    log::info!("Cooking shader: {}", source_url.as_str());

    let shader_source = assets::download_string(&source_url)
        .await
        .map_err(|err| format!("Failed to get source conent: {:?}", err))?;

    let ext = source_url.extension();
    let hash = content_hash::sha256_bytes(shader_source.as_bytes());
    let target_url = target_base
        .join(&format!("{}.{}_spv", hash, ext))
        .map_err(|err| format!("Invalid target url: {:?}", err))?;
    log::trace!("Cooking as: {}", target_url.as_str());

    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    let ty = match ext {
        "vs" => shaderc::ShaderKind::Vertex,
        "fs" => shaderc::ShaderKind::Fragment,
        "cs" => shaderc::ShaderKind::Compute,
        _ => return Err(format!("Unknown shader type: {}", ext)),
    };
    log::trace!("Compiling source");
    let compiled_artifact = compiler
        .compile_into_spirv(&shader_source, ty, source_url.as_str(), "main", Some(&options))
        .map_err(|err| format!("Shader compilation failed: {:?}", err))?;

    log::trace!("Saving result");
    assets::upload_binary(&target_url, compiled_artifact.as_binary_u8())
        .await
        .map_err(|err| format!("Failed to upload {}: {:?}", target_url.as_str(), err))?;

    log::trace!("Cooking completed for {}: {}", source_url.as_str(), target_url.as_str());

    Ok(())
}

async fn run() {
    let config = config::Config::new().unwrap();
    let asset_source_base = Url::parse(&config.asset_source_base).unwrap();
    let asset_target_base = Url::parse(&config.asset_target_base).unwrap();

    if let Err(err) = cook_shader(&asset_source_base, &asset_target_base, "pipeline/hello.vs").await {
        log::error!("{}", err);
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
