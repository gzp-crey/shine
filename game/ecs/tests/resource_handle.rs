use shine_ecs::resources::{ResourceId, Resources};
use std::cell::RefCell;
use std::rc::Rc;

mod utils;

#[derive(Copy, Clone)]
enum TestCase {
    Happy,
    Panic1,
    Panic2,
    Panic3,
}

fn handle_test_core(case: TestCase) {
    utils::init_logger();

    #[derive(Debug)]
    struct TestOne(String, usize);
    impl TestOne {
        #[inline]
        fn assert_eq(&self, s: &str, v: usize) {
            assert_eq!(self.0, s);
            assert_eq!(self.1, v);
        }
    }

    let ida = ResourceId::from_tag("a").unwrap();
    let idb = ResourceId::from_tag("b").unwrap();

    let build_counter = Rc::new(RefCell::new(0));

    let mut resources = Resources::default();
    resources.register::<TestOne, _, _>(
        {
            let cnt = build_counter.clone();
            move |id| {
                *cnt.borrow_mut() += 1;
                TestOne(format!("one {:?}", id), *cnt.borrow())
            }
        },
        |_| {},
    );

    resources
        .get_with_id::<TestOne>(&ida)
        .unwrap()
        .assert_eq("one Tag(SmallStringId(\"a\"))", 1);
    resources
        .get_mut_with_id::<TestOne>(&ida)
        .unwrap()
        .assert_eq("one Tag(SmallStringId(\"a\"))", 1);

    let ha = resources.get_handle::<TestOne>(&ida).unwrap();
    resources.at(&ha).unwrap().assert_eq("one Tag(SmallStringId(\"a\"))", 1);

    {
        // create now, counter incremented
        let hb = resources.get_handle::<TestOne>(&idb).unwrap();
        resources.at(&hb).unwrap().assert_eq("one Tag(SmallStringId(\"b\"))", 2);
        resources
            .get_with_id::<TestOne>(&idb)
            .unwrap()
            .assert_eq("one Tag(SmallStringId(\"b\"))", 2);

        {
            let store_one = resources.get_store::<TestOne>().unwrap();

            // handle and store keeps no lock on resoource
            resources
                .get_mut_with_id::<TestOne>(&ida)
                .unwrap()
                .assert_eq("one Tag(SmallStringId(\"a\"))", 1);

            {
                // read resource a
                let res_a = store_one.at(&ha).unwrap();
                res_a.assert_eq("one Tag(SmallStringId(\"a\"))", 1);
                // read + read is ok
                resources
                    .get_with_id::<TestOne>(&ida)
                    .unwrap()
                    .assert_eq("one Tag(SmallStringId(\"a\"))", 1);
                if let TestCase::Panic1 = case {
                    // read + write is panic
                    let _ = resources.get_mut_with_id::<TestOne>(&ida).unwrap();
                    unreachable!()
                }

                // write resource b
                let res_b = store_one.at_mut(&hb).unwrap();
                res_b.assert_eq("one Tag(SmallStringId(\"b\"))", 2);
                // write + read is panic
                if let TestCase::Panic2 = case {
                    let _ = resources.get_with_id::<TestOne>(&idb).unwrap();
                    unreachable!()
                }
                if let TestCase::Panic3 = case {
                    // write + write
                    let _ = resources.get_mut_with_id::<TestOne>(&idb).unwrap();
                    unreachable!()
                }
            }
        }
    }

    // garbage collect
    resources.get_store_mut::<TestOne>().unwrap().bake();

    //handle was kept alive, no change in version
    resources.at(&ha).unwrap().assert_eq("one Tag(SmallStringId(\"a\"))", 1);

    //handle was dropped, change in version
    let hb = resources.get_handle::<TestOne>(&idb).unwrap();
    resources.at(&hb).unwrap().assert_eq("one Tag(SmallStringId(\"b\"))", 3);
}

#[test]
fn handle_test() {
    handle_test_core(TestCase::Happy);
}

#[test]
#[should_panic(expected = "Resource of resource_handle::handle_test_core::TestOne already borrowed as immutable")]
fn handle_test_fail_1() {
    handle_test_core(TestCase::Panic1);
}

#[test]
#[should_panic(expected = "Resource of resource_handle::handle_test_core::TestOne already borrowed as mutable")]
fn handle_test_fail_2() {
    handle_test_core(TestCase::Panic2);
}

#[test]
#[should_panic(expected = "Resource of resource_handle::handle_test_core::TestOne already borrowed as mutable")]
fn handle_test_fail_3() {
    handle_test_core(TestCase::Panic3);
}
