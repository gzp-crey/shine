#![cfg(feature = "cook")]
use image::GenericImageView;
use shine_game::assets::{io::HashableContent, AssetIO, AssetId, ImageEncoding, TextureSource, Url};
use std::collections::HashMap;

mod utils;

#[tokio::test(threaded_scheduler)]
async fn load_texture_no_meta() {
    utils::init_logger();

    let source_root = Url::parse("file://../assets/game_test/assets/").unwrap();
    let virtual_schemes = HashMap::default();
    let io = AssetIO::new(virtual_schemes).unwrap();

    let id = AssetId::new("image_no_meta.jpg").unwrap();
    let url = id.to_url(&source_root).unwrap();

    let (source, source_hash) = TextureSource::load(&io, &url).await.unwrap();
    assert_eq!(source.image.dimensions(), (640, 426));
    assert_eq!(source.descriptor.image.encoding, ImageEncoding::Png);
    assert_eq!(
        source_hash,
        "8b722f621e425cbefddd3e7d76c168dd1e8216824474c1b505f36634dd70adba"
    );

    let cooked = source.cook().await.unwrap();
    let cooked_hash = bincode::serialize(&cooked).unwrap().content_hash();
    assert_eq!(cooked.image_descriptor.size, (640, 426));
    assert_eq!(cooked.image_descriptor.encoding, ImageEncoding::Png);
    assert_eq!(
        cooked_hash,
        "2884883a01b887b3e25de0264d194dbb3023cb77972aa47d77fddff38779fea0"
    );
}

#[tokio::test(threaded_scheduler)]
async fn load_texture_meta() {
    utils::init_logger();

    let source_root = Url::parse("file://../assets/game_test/assets/").unwrap();
    let virtual_schemes = HashMap::default();
    let io = AssetIO::new(virtual_schemes).unwrap();

    let id = AssetId::new("image_meta.jpg").unwrap();
    let url = id.to_url(&source_root).unwrap();

    let (source, source_hash) = TextureSource::load(&io, &url).await.unwrap();
    assert_eq!(source.image.dimensions(), (640, 426));
    assert_eq!(source.descriptor.image.encoding, ImageEncoding::Jpeg);
    assert_eq!(
        source_hash,
        "f9b78a1f4498e34a9371b01dc4f4ce128cb2c6c6f2d89ff0221707523eba5066"
    );

    let cooked = source.cook().await.unwrap();
    let cooked_hash = bincode::serialize(&cooked).unwrap().content_hash();
    assert_eq!(cooked.image_descriptor.size, (128, 128));
    assert_eq!(cooked.image_descriptor.encoding, ImageEncoding::Jpeg);
    assert_eq!(
        cooked_hash,
        "d5d51b071b46017a15c30f8216a8d8b094c78e11a2aeba543226bab5b231b3e8"
    );
}
