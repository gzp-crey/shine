#![cfg(feature = "cook")]
use shine_game::assets::{io::HashableContent, AssetIO, AssetId, Url};
use shine_game::game::test1;
use std::collections::HashMap;

mod utils;

async fn load_game(id: &str, expected_source_hash: &str, expected_cooked_hash: &str) {
    utils::init_logger();

    let source_root = Url::parse("file://../assets/games").unwrap();
    let virtual_schemes = HashMap::default();
    let io = AssetIO::new(virtual_schemes).unwrap();

    let id = AssetId::new(id).unwrap();
    let source_url = id.to_url(&source_root).unwrap();

    let (source, source_hash) = test1::Source::load(&io, &source_url).await.unwrap();
    //assert_eq!(source.ty, ShaderType::Fragment);
    assert_eq!(source_hash, expected_source_hash);

    let cooked = source.cook(/*cooker::DummyCooker*/).await.unwrap();
    let cooked_hash = bincode::serialize(&cooked).unwrap().content_hash();
    //assert_eq!(cooked.ty, ShaderType::Fragment);
    assert_eq!(cooked_hash, expected_cooked_hash);
}

#[tokio::test(threaded_scheduler)]
async fn load_test1() {
    load_game("test1/test.game", "", "").await;
}
