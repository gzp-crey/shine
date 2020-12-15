#![cfg(feature = "cook")]
use shine_game::assets::{cooker, AssetIO, AssetId, ContentHash, PipelineSource, Url};
use std::collections::HashMap;

mod utils;

#[tokio::test(threaded_scheduler)]
async fn load_pipeline() {
    utils::init_logger();

    let source_root = Url::parse("file://../assets/game_test/").unwrap();
    let virtual_schemes = HashMap::default();
    let io = AssetIO::new(virtual_schemes).unwrap();

    let id = AssetId::new("hello.pl").unwrap();
    let source_url = id.to_url(&source_root).unwrap();

    let (source, source_hash) = PipelineSource::load(&io, &id, &source_url).await.unwrap();
    assert_eq!(
        source_hash.hash(),
        "82556a847da246efac991053b60615df69fb90f58af4e1642c18a2d4fb6017dd"
    );

    let cooked = source.cook(cooker::DummyCooker).await.unwrap();
    log::debug!("cooked descriptor: {:#?}", cooked.descriptor);
    let cooked_hash = ContentHash::from_bytes(&bincode::serialize(&cooked).unwrap());
    assert_eq!(
        cooked.descriptor.vertex_stage.shader,
        "hash-shader://ec10/bf5ed69cc5a1c1fc87607781c9b9.vs"
    );
    assert_eq!(
        cooked.descriptor.fragment_stage.shader,
        "hash-shader://d9a8/45d3f0efcc273c98913362da3f08.fs"
    );
    assert_eq!(
        cooked_hash.hash(),
        "332df9317060ff78d282f128bd429fab2d4715d1a20cd8fa9ea9f34e166416b9"
    );
}
