#![cfg(off)]
#![cfg(feature = "cook")]
use shine_game::assets::{io::HashableContent, AssetIO, AssetId, PipelineSource, Url};
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

    let (source, source_hash) = PipelineSource::load(&io, &source_url).await.unwrap();
    //assert_eq!(source.ty, ShaderType::Fragment);
    assert_eq!(
        source_hash,
        "a887bf72cbe211e2dcedbc7b551f3cdc435ba051d6af3851570c8f15de6aa09c"
    );

    let cooked = source.cook().await.unwrap();
    let cooked_hash = bincode::serialize(&cooked).unwrap().content_hash();
    //assert_eq!(cooked.ty, ShaderType::Fragment);
    assert_eq!(
        cooked_hash,
        "9a7502469c43061835153ced78b5eae1639d3e798440bd4f3ca20cbd4e504f24"
    );
}
