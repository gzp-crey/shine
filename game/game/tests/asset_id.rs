#![cfg(feature = "cook")]
use shine_game::assets::AssetId;

mod utils;

fn test_split_folder(src: &str, tgt: (Option<&str>, &str)) {
    assert_eq!(AssetId::new(src).unwrap().split_folder(), tgt);
}

fn test_set_extension(src: &str, ext: &str, tgt: &str) {
    assert_eq!(AssetId::new(src).unwrap().set_extension(ext).unwrap().as_str(), tgt);
}

fn test_extension(src: &str, ext: &str) {
    assert_eq!(AssetId::new(src).unwrap().extension(), ext);
}

#[tokio::test(threaded_scheduler)]
async fn test_asset_id() {
    utils::init_logger();

    test_split_folder("global/something/alma", (Some("global/something"), "alma"));
    test_split_folder("global/something/alma.txt", (Some("global/something"), "alma.txt"));
    test_split_folder("./alma.txt", (Some("."), "alma.txt"));
    test_split_folder("./alma.", (Some("."), "alma."));
    test_split_folder("./alma", (Some("."), "alma"));
    test_split_folder("alma", (None, "alma"));
    test_split_folder("/alma", (Some(""), "alma"));
    test_split_folder("/", (Some(""), ""));
    test_split_folder("", (None, ""));

    test_set_extension("./alma.txt", "ch", "./alma.ch");
    test_set_extension("./alma.", "ch", "./alma.ch");
    test_set_extension("./alma", "ch", "./alma.ch");
    test_set_extension("alma.txt", "ch", "alma.ch");
    test_set_extension("alma.", "ch", "alma.ch");
    test_set_extension("alma", "ch", "alma.ch");
    test_set_extension("./.txt", "ch", "./.ch");
    test_set_extension("./.", "ch", "./.ch");
    test_set_extension("./", "ch", "./.ch");
    test_set_extension("global/something/alma.txt", "ch", "global/something/alma.ch");
    test_set_extension("global/something/alma.", "ch", "global/something/alma.ch");
    test_set_extension("global/something/alma", "ch", "global/something/alma.ch");
    test_set_extension("global/somet.hing/alma.txt", "ch", "global/somet.hing/alma.ch");
    test_set_extension("global/somet.hing/alma.", "ch", "global/somet.hing/alma.ch");
    test_set_extension("global/somet.hing/alma", "ch", "global/somet.hing/alma.ch");

    test_extension("./alma.txt", "txt");
    test_extension("./alma.", "");
    test_extension("./alma", "");
    test_extension("alma.txt", "txt");
    test_extension("alma.", "");
    test_extension("alma", "");
    test_extension("./.txt", "txt");
    test_extension("./.", "");
    test_extension(".txt", "txt");
    test_extension(".", "");
    test_extension("global/something/alma.ext", "ext");
    test_extension("global/something/alma.", "");
    test_extension("global/something/alma", "");
    test_extension("global/somet.hing/alma.txt", "txt");
    test_extension("global/somet.hing/alma.", "");
    test_extension("global/somet.hing/alma", "");
}
