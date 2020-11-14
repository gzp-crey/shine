use shine_ecs::resources::{ManagedResource, ResourceId, Resources};
use std::cell::RefCell;
use std::rc::Rc;

mod utils;

#[derive(Debug)]
struct TestOne(String, usize);

impl TestOne {
    #[inline]
    fn assert_eq(&self, s: &str, v: usize) {
        assert_eq!(self.0, s);
        assert_eq!(self.1, v);
    }
}

#[derive(Copy, Clone)]
enum TestCase {
    Happy,
    Panic1,
    Panic2,
    Panic3,
}

fn handle_test_core(case: TestCase) {
    utils::init_logger();

    let ida = ResourceId::from_tag("a").unwrap();
    let idb = ResourceId::from_tag("b").unwrap();

    let build_counter = Rc::new(RefCell::new(0));

    let mut resources = Resources::default();

    resources.register(ManagedResource::new(true, {
        let cnt = build_counter.clone();
        move |id| {
            *cnt.borrow_mut() += 1;
            TestOne(format!("one {:?}", id), *cnt.borrow())
        }
    }));

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
#[should_panic(
    expected = "Mutable borrow of a resource [resource_handle::TestOne] failed: Target already borrowed as immutable"
)]
fn handle_test_fail_1() {
    handle_test_core(TestCase::Panic1);
}

#[test]
#[should_panic(
    expected = "Immutable borrow of a resource [resource_handle::TestOne] failed: Target already borrowed as mutable"
)]
fn handle_test_fail_2() {
    handle_test_core(TestCase::Panic2);
}

#[test]
#[should_panic(
    expected = "Mutable borrow of a resource [resource_handle::TestOne] failed: Target already borrowed as mutable"
)]
fn handle_test_fail_3() {
    handle_test_core(TestCase::Panic3);
}

#[test]
#[cfg(TODO)]
fn simple_multi_threaded() {
    utils::init_logger();
    utils::single_threaded_test();

    const ITER: u32 = 10;
    let store = store::no_load::<TestData>(2);

    // request from multiple threads
    let store = Arc::new(store);
    {
        let mut tp = vec![];
        for i in 0..ITER {
            let store = store.clone();
            tp.push(thread::spawn(move || {
                let store = store.try_read().unwrap();
                assert!(store.try_get(&TestDataId(0)) == None);

                // request 1
                let r1 = store.get_or_add(&TestDataId(1));
                assert!(store.at(&r1).0 == format!("id: {}", 1));

                // request 100 + threadId
                let r100 = store.get_or_add(&TestDataId(100 + i));
                assert!(store.at(&r100).0 == format!("id: {}", 100 + i));

                for _ in 0..100 {
                    assert!(store.at(&r1).0 == format!("id: {}", 1));
                    assert!(store.at(&r100).0 == format!("id: {}", 100 + i));
                }
            }));
        }
        for t in tp.drain(..) {
            t.join().unwrap();
        }
    }

    log::info!("request process");
    let mut store = Arc::try_unwrap(store).map_err(|_| ()).unwrap();
    store.finalize_requests();

    // check after process
    let store = Arc::new(store);
    {
        let mut tp = vec![];
        for i in 0..ITER {
            let store = store.clone();
            tp.push(thread::spawn(move || {
                let store = store.try_read().unwrap();
                assert!(store.try_get(&TestDataId(0)) == None);

                // get 1
                let r1 = store.try_get(&TestDataId(1)).unwrap();
                assert!(store.at(&r1).0 == format!("id: {}", 1));

                // get 100 + threadId
                let r100 = store.try_get(&TestDataId(100 + i)).unwrap();
                assert!(store.at(&r100).0 == format!("id: {}", 100 + i));
            }));
        }
        for t in tp.drain(..) {
            t.join().unwrap();
        }
    }

    log::info!("drain");
    let mut store = Arc::try_unwrap(store).map_err(|_| ()).unwrap();
    store.finalize_requests();
    store.drain_unused();

    // check after drain
    let store = Arc::new(store);
    {
        let mut tp = vec![];
        for i in 0..ITER {
            let store = store.clone();
            tp.push(thread::spawn(move || {
                let store = store.try_read().unwrap();
                assert!(store.try_get(&TestDataId(0)) == None);

                // get 1
                assert!(store.try_get(&TestDataId(1)) == None);

                // get 100 + threadId
                assert!(store.try_get(&TestDataId(100 + i)) == None);
            }));
        }
        for t in tp.drain(..) {
            t.join().unwrap();
        }
    }
}
