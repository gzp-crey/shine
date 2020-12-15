#![cfg(feature = "cook")]
use shine_game::assets::{cooker, io::HashableContent, AssetIO, AssetId, PipelineSource, Url};
use std::collections::HashMap;

mod utils;

#[tokio::test(threaded_scheduler)]
async fn load_pipeline() {
    utils::init_logger();

    let source_root = Url::parse("file://../assets/game_test/assets/").unwrap();
    let virtual_schemes = HashMap::default();
    let io = AssetIO::new(virtual_schemes).unwrap();

    let id = AssetId::new("hello.pl").unwrap();
    let source_url = id.to_url(&source_root).unwrap();

    let (source, source_hash) = PipelineSource::load(&io, &id, &source_url).await.unwrap();
    //assert_eq!(source.ty, ShaderType::Fragment);
    assert_eq!(
        source_hash,
        "82556a847da246efac991053b60615df69fb90f58af4e1642c18a2d4fb6017dd"
    );

    let cooked = source.cook(cooker::DummyCooker).await.unwrap();
    log::debug!("cooked descriptor: {:#?}", cooked.descriptor);
    let cooked_hash = bincode::serialize(&cooked).unwrap().content_hash();
    assert_eq!(cooked.descriptor.vertex_stage.shader, "shader://hello.vs");
    assert_eq!(cooked.descriptor.fragment_stage.shader, "shader://hello.fs");
    assert_eq!(
        cooked_hash,
        "39c2fd987c34f732c243867cf197c8586aaee1788df5564cc1b5cbb7793c7abc"
    );
}
