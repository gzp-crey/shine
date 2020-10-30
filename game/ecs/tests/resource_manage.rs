use shine_ecs::resources::{ResourceHandle, ResourceId, ResourceTag, Resources};
use std::str::FromStr;

mod utils;

#[derive(PartialEq)]
enum TestCase {
    Happy,
}

fn handle_test_core(case: TestCase) {
    utils::init_logger();

    #[derive(Debug)]
    struct TestOne(String);
    #[derive(Debug)]
    struct TestTwo(String);

    let ida = ResourceId::Tag(ResourceTag::from_str("a").unwrap());
    let idb = ResourceId::Tag(ResourceTag::from_str("b").unwrap());
    let idc = ResourceId::Tag(ResourceTag::from_str("c").unwrap());

    let mut counter_a = 0;
    let mut counter_b = 0;

    let mut resources = Resources::default();
    resources.insert_managed::<TestOne, _>(|id| {
        counter_a += 1;
        TestOne(format!("one {:?}", id))
    });
    resources.insert_managed::<TestTwo, _>(|id| {
        counter_b += 1;
        TestTwo(format!("two {:?}", id))
    });

    assert_eq!(counter_a, 0);
    log::info!("{:?}", resources.get_with_id::<TestOne>(&ida));
    assert_eq!(counter_a, 1);
    let ha = resources.handle::<TestOne>(&ida); // already created
    assert_eq!(counter_a, 1);
    let hb = resources.handle::<TestOne>(&idb); // create now
    assert_eq!(counter_a, 2);
    log::info!("{:?}", resources.get_with_id::<TestOne>(&ida)); // already created
    assert_eq!(counter_a, 2);
}

#[test]
fn handle_test() {
    handle_test_core(TestCase::Happy);
}
