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
    let uniform_layout = source.descriptor.get_uniform_layout().unwrap();
    log::debug!("uniform_layout: {:#?}", uniform_layout);
    assert_eq!(uniform_layout.len(), 4);
    assert!(uniform_layout[0].is_empty());
    assert!(uniform_layout[1].is_empty());
    assert_eq!(
        source_hash.hash(),
        "39d34ac02c5543b2f379acc4f7f2f03b994be11297b533bc3e75e087b1247448"
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
        "9cfcff89ca030161cbf52abbaf186c24e8ef40e526a4f325e640a7e170c607e3"
    );
}
