use shine_ecs::resources::{Resource, ResourceId, ResourceTag, Resources, UnmanagedResource};
use std::str::FromStr;

mod utils;

struct TestOne(String);
impl Resource for TestOne {
    type Config = UnmanagedResource<Self>;
}

struct TestTwo(String);
impl Resource for TestTwo {
    type Config = UnmanagedResource<Self>;
}

struct NotSync(*const u8);
impl Resource for NotSync {
    type Config = UnmanagedResource<Self>;
}

#[derive(Copy, Clone)]
enum SimpleTestCase {
    Happy,
    Panic1,
    Panic2,
}

fn simple_test_core(case: SimpleTestCase) {
    utils::init_logger();

    let id = ResourceId::Tag(ResourceTag::from_str("ptr").unwrap());
    let gid = ResourceId::Global;

    let mut resources = Resources::default();

    resources.register::<TestOne>(Default::default());
    resources.register::<TestTwo>(Default::default());
    resources.register::<NotSync>(Default::default());

    resources.insert(TestOne("one".to_string())).unwrap();
    resources.insert(TestTwo("two".to_string())).unwrap();
    resources.insert_with_id(id.clone(), NotSync(std::ptr::null())).unwrap();

    assert!(resources.get_store::<TestOne>().unwrap().contains(&gid));
    assert!(resources.get_store::<TestTwo>().unwrap().contains(&gid));
    assert!(resources.get_store::<NotSync>().unwrap().contains(&id));

    assert!(resources.get_store_mut::<TestOne>().unwrap().contains(&gid));
    assert_eq!(resources.get::<TestOne>().unwrap().0, "one");

    assert!(resources.get_store_mut::<TestTwo>().unwrap().contains(&gid));
    assert_eq!(resources.get::<TestTwo>().unwrap().0, "two");

    assert!(resources.get_store_mut::<NotSync>().unwrap().contains(&id));
    assert_eq!(resources.get_with_id::<NotSync>(&id).unwrap().0, std::ptr::null());

    {
        let test_one_store = resources.get_store::<TestOne>().unwrap();
        let test_two_store = resources.get_store::<TestTwo>().unwrap();
        let not_sync_store = resources.get_store::<NotSync>().unwrap();

        if let SimpleTestCase::Panic1 = case {
            // should panic
            let _ = resources.get_store_mut::<TestTwo>().unwrap();
            unreachable!()
        }

        assert!(test_one_store.contains(&gid));
        assert_eq!(test_one_store.get().unwrap().0, "one");
        assert_eq!(resources.get::<TestOne>().unwrap().0, "one");
        assert_eq!(resources.get_mut::<TestOne>().unwrap().0, "one");

        if let SimpleTestCase::Panic2 = case {
            let _r = resources.get::<TestOne>().unwrap();
            let _ = resources.get_mut::<TestOne>(); // should panic
        }

        assert!(test_two_store.contains(&gid));
        assert_eq!(test_two_store.get().unwrap().0, "two");
        assert_eq!(resources.get::<TestTwo>().unwrap().0, "two");

        assert!(not_sync_store.contains(&id));
        assert_eq!(not_sync_store.get_with_id(&id).unwrap().0, std::ptr::null());
        assert_eq!(resources.get_with_id::<NotSync>(&id).unwrap().0, std::ptr::null());
    }

    // test re-ownership
    let owned = resources.remove::<TestTwo>();
    assert_eq!(owned.unwrap().0, "two");
}

#[test]
fn simple_test() {
    simple_test_core(SimpleTestCase::Happy);
}

#[test]
#[should_panic(expected = "Resource store for resource_access::TestTwo: AlreadyReadLocked")]
fn simple_test_fail_1() {
    simple_test_core(SimpleTestCase::Panic1);
}

#[test]
#[should_panic(expected = "Resource of resource_access::TestOne: AlreadyReadLocked")]
fn simple_test_fail_2() {
    simple_test_core(SimpleTestCase::Panic2);
}

#[derive(Copy, Clone)]
enum MultiTestCase {
    Happy,
    Panic1,
    Panic2,
}

fn multi_test_core(case: MultiTestCase) {
    utils::init_logger();

    let mut resources = Resources::default();

    resources.register::<TestOne>(Default::default());
    resources.register::<TestTwo>(Default::default());

    let ida = ResourceId::from_tag("a").unwrap();
    let idb = ResourceId::from_tag("b").unwrap();

    resources.insert(TestOne("one".to_string())).unwrap();
    resources
        .insert_with_id(ida.clone(), TestOne("one_a".to_string()))
        .unwrap();
    resources
        .insert_with_id(idb.clone(), TestOne("one_b".to_string()))
        .unwrap();

    resources.insert(TestTwo("two".to_string())).unwrap();
    resources
        .insert_with_id(ida.clone(), TestTwo("two_a".to_string()))
        .unwrap();
    resources
        .insert_with_id(idb.clone(), TestTwo("two_b".to_string()))
        .unwrap();

    assert_eq!(resources.get::<TestOne>().unwrap().0, "one");
    assert_eq!(resources.get_with_id::<TestOne>(&ida).unwrap().0, "one_a");

    assert_eq!(resources.get::<TestTwo>().unwrap().0, "two");
    assert_eq!(resources.get_with_id::<TestTwo>(&idb).unwrap().0, "two_b");

    let test_one_store = resources.get_store::<TestOne>().unwrap();

    {
        log::info!("get after get");
        let test_one_res = resources
            .get_with_ids::<TestOne, _>(&[ida.clone(), idb.clone()])
            .unwrap();
        assert_eq!(test_one_res[0].0, "one_a");
        assert_eq!(test_one_res[1].0, "one_b");
        assert_eq!(resources.get::<TestOne>().unwrap().0, "one");
        assert_eq!(resources.get_with_id::<TestOne>(&ida).unwrap().0, "one_a");
        assert_eq!(resources.get_with_id::<TestOne>(&idb).unwrap().0, "one_b");

        assert_eq!(resources.get_mut::<TestOne>().unwrap().0, "one");
        if let MultiTestCase::Panic1 = case {
            let _ = resources.get_mut_with_id::<TestOne>(&ida);
        }
    }

    {
        log::info!("get after get_mut");

        let test_one_res = test_one_store.get_mut_with_ids(&[ida.clone(), idb.clone()]).unwrap();
        assert_eq!(test_one_res[0].0, "one_a");
        assert_eq!(test_one_res[1].0, "one_b");
        {
            assert_eq!(resources.get::<TestOne>().unwrap().0, "one");
        }

        if let MultiTestCase::Panic2 = case {
            let _ = resources.get_with_id::<TestOne>(&ida);
            unreachable!()
        }
    }
}

#[test]
fn multi_test() {
    multi_test_core(MultiTestCase::Happy);
}

#[test]
#[should_panic(expected = "Resource of resource_access::TestOne: AlreadyReadLocked")]
fn multi_test_core_fail_1() {
    multi_test_core(MultiTestCase::Panic1);
}

#[test]
#[should_panic(expected = "Resource of resource_access::TestOne: AlreadyWriteLocked")]
fn multi_test_core_fail_2() {
    multi_test_core(MultiTestCase::Panic2);
}
