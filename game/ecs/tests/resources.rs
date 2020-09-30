use shine_ecs::resources::Resources;

mod utils;

#[test]
fn simple_read_write_test() {
    utils::init_logger();

    struct TestOne {
        value: String,
    }

    struct TestTwo {
        value: String,
    }

    struct NotSync {
        ptr: *const u8,
    }

    let mut resources = Resources::default();
    resources.insert(TestOne {
        value: "one".to_string(),
    });

    resources.insert(TestTwo {
        value: "two".to_string(),
    });

    let name = ResourceName::from_str("ptr").unwrap();
    resources.insert_with_name(name.clone(), NotSync { ptr: std::ptr::null() });

    assert_eq!(resources.get::<TestOne>().unwrap().value, "one");
    assert_eq!(resources.get::<TestTwo>().unwrap().value, "two");
    assert_eq!(resources.get_with_name::<NotSync>(&name).unwrap().ptr, std::ptr::null());

    // test re-ownership
    let owned = resources.remove::<TestTwo>();
    assert_eq!(owned.unwrap().value, "two");
}
