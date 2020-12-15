#![cfg(feature = "cook")]
use shine_game::assets::{io::HashableContent, AssetIO, AssetId, ShaderSource, ShaderType, Url};
use std::collections::HashMap;

mod utils;

#[tokio::test(threaded_scheduler)]
async fn load_shader() {
    utils::init_logger();

    let source_root = Url::parse("file://../assets/game_test/assets/").unwrap();
    let virtual_schemes = HashMap::default();
    let io = AssetIO::new(virtual_schemes).unwrap();

    let id = AssetId::new("hello.fs").unwrap();
    let source_url = id.to_url(&source_root).unwrap();

    let (source, source_hash) = ShaderSource::load(&io, &id, &source_url).await.unwrap();
    assert_eq!(source.shader_type, ShaderType::Fragment);
    assert_eq!(
        source_hash,
        "adcdb2c2b8e24b2f83e3a76c3139cf445a591f00194e1c6f80bb0852c7100d95"
    );

    let cooked = source.cook().await.unwrap();
    let cooked_hash = bincode::serialize(&cooked).unwrap().content_hash();
    assert_eq!(cooked.shader_type, ShaderType::Fragment);
    assert_eq!(
        cooked_hash,
        "9a7502469c43061835153ced78b5eae1639d3e798440bd4f3ca20cbd4e504f24"
    );
}
