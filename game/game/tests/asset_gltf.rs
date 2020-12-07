#![cfg(feature = "cook")]
use shine_game::assets::{io::HashableContent, AssetIO, AssetId, GltfSource, Url};
use std::collections::HashMap;

mod utils;

#[tokio::test(threaded_scheduler)]
async fn load_gltf() {
    utils::init_logger();

    let source_root = Url::parse("file://../assets/game_test/assets/").unwrap();
    let virtual_schemes = HashMap::default();
    let io = AssetIO::new(virtual_schemes).unwrap();

    let id = AssetId::new("VertexColorTest.glb").unwrap();
    let source_url = id.to_url(&source_root).unwrap();

    let (source, source_hash) = GltfSource::load(&io, &source_url).await.unwrap();
    //assert_eq!(source., );
    assert_eq!(
        source_hash,
        "58006bdcff8084339da0f6e24400160890638c16dbcb83c362ccaf150e8c6e10"
    );

    let cooked = source.cook().await.unwrap();
    let cooked_hash = bincode::serialize(&cooked).unwrap().content_hash();
    //assert_eq!(cooked., );
    assert_eq!(
        cooked_hash,
        "48a50c9255cbff5aaea2bec95815fd1684ae3186854d66c1ea1b3acd03a4b9ec"
    );
}
