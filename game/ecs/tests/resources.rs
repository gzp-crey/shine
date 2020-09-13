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
    resources.insert(
        None,
        TestOne {
            value: "one".to_string(),
        },
    );

    resources.insert(
        None,
        TestTwo {
            value: "two".to_string(),
        },
    );

    resources.insert(None, NotSync { ptr: std::ptr::null() });

    assert_eq!(resources.get::<TestOne>(&None).unwrap().value, "one");
    assert_eq!(resources.get::<TestTwo>(&None).unwrap().value, "two");
    assert_eq!(resources.get::<NotSync>(&None).unwrap().ptr, std::ptr::null());

    // test re-ownership
    let owned = resources.remove::<TestTwo>(&None);
    assert_eq!(owned.unwrap().value, "two");
}
