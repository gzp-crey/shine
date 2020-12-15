#![cfg(feature = "cook")]
use shine_game::{
    assets::{cooker, AssetIO, AssetId, ContentHash, Url},
    game::test1,
};
use std::collections::HashMap;

mod utils;

#[tokio::test(threaded_scheduler)]
async fn load_test1() {
    utils::init_logger();

    let source_root = Url::parse("file://../assets/game_test/").unwrap();
    let virtual_schemes = HashMap::default();
    let io = AssetIO::new(virtual_schemes).unwrap();

    let id = AssetId::new("test1.game").unwrap();
    let source_url = id.to_url(&source_root).unwrap();

    let (source, source_hash) = test1::Source::load(&io, &id, &source_url).await.unwrap();
    assert_eq!(
        source_hash.hash(),
        "ec71cc1b4a27f80ccd32673a5b06abb7ef576ce5d7c6fb2575a660c406d8aa05"
    );

    let cooked = source.cook(cooker::DummyCooker).await.unwrap();
    assert_eq!(cooked.pipeline, "pipeline://hello.pl");
    let cooked_hash = ContentHash::from_bytes(&bincode::serialize(&cooked).unwrap());
    assert_eq!(
        cooked_hash.hash(),
        "00677fe9400b8f54d7c11bb361bce9eedf19c413389a04a0c7bf243dbebe9d08"
    );
}

#[tokio::test(threaded_scheduler)]
async fn load_test1_invalid() {
    utils::init_logger();

    let source_root = Url::parse("file://../assets/game_test/").unwrap();
    let virtual_schemes = HashMap::default();
    let io = AssetIO::new(virtual_schemes).unwrap();

    let id = AssetId::new("test1_invalid.game").unwrap();
    let source_url = id.to_url(&source_root).unwrap();

    let err = test1::Source::load(&io, &id, &source_url)
        .await
        .map(|_| ())
        .unwrap_err();
    assert!(format!("{:?}", err).contains("unknown variant `Test2`, expected `Test1`"));
}
