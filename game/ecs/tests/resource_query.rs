use shine_ecs::resources::{ResMutQuery, ResQuery, Resources};

mod utils;

#[derive(Debug)]
struct TestOne {
    s: String,
}

#[derive(Debug)]
struct TestTwo {
    s: String,
}

#[test]
fn simple_test_core() {
    utils::init_logger();

    let mut resources = Resources::default();

    resources.register_unmanaged::<TestOne>().unwrap();
    resources.insert(TestOne { s: "one".to_owned() }).unwrap();

    resources.register_unmanaged::<TestTwo>().unwrap();
    resources.insert(TestTwo { s: "two".to_owned() }).unwrap();

    let mut q1 = ResMutQuery::<TestOne>::new();
    let mut q2 = ResQuery::<TestTwo>::new();
    let r1 = resources.claim(&mut q1).unwrap();
    let r2 = resources.claim(&mut q2).unwrap();
    assert_eq!(r1.s, "one");
    assert_eq!(r2.s, "two");
}
