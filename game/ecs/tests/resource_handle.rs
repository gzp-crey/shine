use shine_ecs::resources::{ManagedResource, ResourceId, Resources};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

mod utils;

#[derive(Debug)]
struct SimpleTest(String, usize);

impl SimpleTest {
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

fn simple_test_core(case: TestCase) {
    utils::init_logger();

    let ida = ResourceId::from_tag("a").unwrap();
    let idb = ResourceId::from_tag("b").unwrap();

    let build_counter = Rc::new(RefCell::new(0));

    let mut resources = Resources::default();

    resources
        .register(ManagedResource::new({
            let cnt = build_counter.clone();
            move |id| {
                *cnt.borrow_mut() += 1;
                SimpleTest(format!("one {:?}", id), *cnt.borrow())
            }
        }))
        .unwrap();

    resources
        .get_with_id::<SimpleTest>(&ida)
        .unwrap()
        .assert_eq("one Tag(SmallStringId(\"a\"))", 1);
    resources
        .get_mut_with_id::<SimpleTest>(&ida)
        .unwrap()
        .assert_eq("one Tag(SmallStringId(\"a\"))", 1);

    let ha = resources.get_handle::<SimpleTest>(&ida).unwrap();
    resources.at(&ha).assert_eq("one Tag(SmallStringId(\"a\"))", 1);

    {
        // create now, counter incremented
        let hb = resources.get_handle::<SimpleTest>(&idb).unwrap();
        resources.at(&hb).assert_eq("one Tag(SmallStringId(\"b\"))", 2);
        resources
            .get_with_id::<SimpleTest>(&idb)
            .unwrap()
            .assert_eq("one Tag(SmallStringId(\"b\"))", 2);

        {
            let store_one = resources.get_store::<SimpleTest>().unwrap();

            // handle and store keeps no lock on resoource
            resources
                .get_mut_with_id::<SimpleTest>(&ida)
                .unwrap()
                .assert_eq("one Tag(SmallStringId(\"a\"))", 1);

            {
                // read resource a
                let res_a = store_one.at(&ha);
                res_a.assert_eq("one Tag(SmallStringId(\"a\"))", 1);
                // read + read is ok
                resources
                    .get_with_id::<SimpleTest>(&ida)
                    .unwrap()
                    .assert_eq("one Tag(SmallStringId(\"a\"))", 1);
                if let TestCase::Panic1 = case {
                    // read + write is panic
                    let _ = resources.get_mut_with_id::<SimpleTest>(&ida).unwrap();
                    unreachable!()
                }

                // write resource b
                let res_b = store_one.at_mut(&hb);
                res_b.assert_eq("one Tag(SmallStringId(\"b\"))", 2);
                // write + read is panic
                if let TestCase::Panic2 = case {
                    let _ = resources.get_with_id::<SimpleTest>(&idb).unwrap();
                    unreachable!()
                }
                if let TestCase::Panic3 = case {
                    // write + write
                    let _ = resources.get_mut_with_id::<SimpleTest>(&idb).unwrap();
                    unreachable!()
                }
            }
        }
    }

    // garbage collect
    resources.get_store_mut::<SimpleTest>().unwrap().bake(true);

    //handle was kept alive, no change in version
    resources.at(&ha).assert_eq("one Tag(SmallStringId(\"a\"))", 1);

    //handle was dropped, change in version
    let hb = resources.get_handle::<SimpleTest>(&idb).unwrap();
    resources.at(&hb).assert_eq("one Tag(SmallStringId(\"b\"))", 3);
}

#[test]
fn simple_test() {
    simple_test_core(TestCase::Happy);
}

#[test]
#[should_panic(
    expected = "Mutable borrow of a resource [resource_handle::SimpleTest] failed: Target already borrowed as immutable"
)]
fn simple_test_fail_1() {
    simple_test_core(TestCase::Panic1);
}

#[test]
#[should_panic(
    expected = "Immutable borrow of a resource [resource_handle::SimpleTest] failed: Target already borrowed as mutable"
)]
fn simple_test_fail_2() {
    simple_test_core(TestCase::Panic2);
}

#[test]
#[should_panic(
    expected = "Mutable borrow of a resource [resource_handle::SimpleTest] failed: Target already borrowed as mutable"
)]
fn simple_test_fail_3() {
    simple_test_core(TestCase::Panic3);
}

struct ThreadedTest {
    value: usize,
    counter: Arc<AtomicUsize>,
}

impl ThreadedTest {
    fn new(counter: Arc<AtomicUsize>, id: &ResourceId) -> Self {
        if let ResourceId::Counter(cnt) = id {
            counter.fetch_add(1, Ordering::Relaxed);
            ThreadedTest { value: *cnt, counter }
        } else {
            panic!("invalid id: {:?}", id)
        }
    }
}

impl Drop for ThreadedTest {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::Relaxed);
    }
}

#[test]
fn threaded_test() {
    utils::init_logger();
    //utils::single_threaded_test();

    const THREAD_COUNT: usize = 100;
    const OFFSET: usize = 10000;
    const OFFSET2: usize = OFFSET * 10;

    let counter = Arc::new(AtomicUsize::new(0));
    let resources = {
        let mut resources = Resources::default();

        resources
            .register(ManagedResource::new({
                let counter = counter.clone();
                move |id| ThreadedTest::new(counter.clone(), id)
            }))
            .unwrap();
        resources
    };

    log::info!(
        "create resources ({}) on {} threads",
        THREAD_COUNT + 1,
        rayon::current_num_threads()
    );
    {
        let res = resources.sync();
        rayon::scope(|s| {
            for thread_id in 0..THREAD_COUNT {
                s.spawn({
                    let res = &res;
                    move |_| {
                        let store = res.get_store::<ThreadedTest>().unwrap();

                        // request 1 from each thread
                        let r_1 = store.get_handle(&ResourceId::from_counter(1)).unwrap();
                        assert_eq!(store.at(&r_1).value, 1);

                        // request "owned" resource: OFFSET + thread_id
                        let r_own = store.get_handle(&ResourceId::from_counter(OFFSET + thread_id)).unwrap();
                        assert_eq!(store.at(&r_own).value, OFFSET + thread_id);

                        // request resources owned by all the other threads, but only if they are created
                        for j in 0..THREAD_COUNT {
                            let id = ResourceId::Counter(OFFSET + j);
                            if store.exists(&id) {
                                assert_eq!(store.get_with_id(&id).unwrap().value, OFFSET + j);
                            }
                        }
                    }
                });
            }
        });
        assert_eq!(counter.load(Ordering::Relaxed), THREAD_COUNT + 1);
    }

    log::info!("bake without gc");
    {
        assert_eq!(counter.load(Ordering::Relaxed), THREAD_COUNT + 1); // no change in resources
        resources.bake::<ThreadedTest>(false);
        assert_eq!(counter.load(Ordering::Relaxed), THREAD_COUNT + 1); // no change in resources
    }

    log::info!("check bake without gc");
    {
        assert_eq!(counter.load(Ordering::Relaxed), THREAD_COUNT + 1); // no change in resources
        let res = resources.sync();
        rayon::scope(|s| {
            for thread_id in 0..THREAD_COUNT {
                s.spawn({
                    let res = &res;
                    move |_| {
                        let store = res.get_store::<ThreadedTest>().unwrap();

                        let id = ResourceId::Counter(1);
                        assert!(store.exists(&id));
                        assert_eq!(store.get_with_id(&id).unwrap().value, 1);

                        let id_own = ResourceId::from_counter(OFFSET + thread_id);
                        let r_own = store.get_handle(&id_own).unwrap();
                        assert_eq!(store.at(&r_own).value, OFFSET + thread_id);

                        for j in 0..THREAD_COUNT {
                            let id = ResourceId::Counter(OFFSET + j);
                            assert!(store.exists(&id), "Resource with id {:?} not exists", id);
                            assert_eq!(store.get_with_id(&id).unwrap().value, OFFSET + j);
                        }
                    }
                });
            }
        });
        assert_eq!(counter.load(Ordering::Relaxed), THREAD_COUNT + 1); // no change in resources
    }

    log::info!("update resources, and create new owned");
    {
        assert_eq!(counter.load(Ordering::Relaxed), THREAD_COUNT + 1); // no change in resources
        let res = resources.sync();
        rayon::scope(|s| {
            for thread_id in 0..THREAD_COUNT {
                s.spawn({
                    let res = &res;
                    move |_| {
                        let store = res.get_store::<ThreadedTest>().unwrap();

                        // update owned
                        let id_own = ResourceId::from_counter(OFFSET + thread_id);
                        let r_own = store.get_handle(&id_own).unwrap();
                        store.at_mut(&r_own).value += OFFSET2;

                        // request "owned2" resource: OFFSET2 + thread_id
                        let r_own2 = store
                            .get_handle(&ResourceId::from_counter(OFFSET2 + thread_id))
                            .unwrap();
                        assert_eq!(store.at(&r_own2).value, OFFSET2 + thread_id);

                        // request resources owned by all the other threads, but only if they are created
                        for j in 0..THREAD_COUNT {
                            let id = ResourceId::Counter(OFFSET2 + j);
                            if store.exists(&id) {
                                assert_eq!(store.get_with_id(&id).unwrap().value, OFFSET2 + j);
                            }
                        }
                    }
                });
            }
        });
        assert_eq!(counter.load(Ordering::Relaxed), THREAD_COUNT + THREAD_COUNT + 1);
        // no change in resources
    }

    log::info!("check update");
    {
        assert_eq!(counter.load(Ordering::Relaxed), THREAD_COUNT + THREAD_COUNT + 1); // no change in resources
        let res = resources.sync();
        rayon::scope(|s| {
            for thread_id in 0..THREAD_COUNT {
                s.spawn({
                    let res = &res;
                    move |_| {
                        let store = res.get_store::<ThreadedTest>().unwrap();

                        let id = ResourceId::Counter(1);
                        assert!(store.exists(&id));
                        assert_eq!(store.get_with_id(&id).unwrap().value, 1);

                        let id_own = ResourceId::from_counter(OFFSET + thread_id);
                        assert_eq!(store.get_with_id(&id_own).unwrap().value, OFFSET2 + OFFSET + thread_id);

                        let id_own2 = ResourceId::from_counter(OFFSET2 + thread_id);
                        assert_eq!(store.get_with_id(&id_own2).unwrap().value, OFFSET2 + thread_id);

                        for j in 0..THREAD_COUNT {
                            let id = ResourceId::Counter(OFFSET + j);
                            assert!(store.exists(&id), "Resource with id {:?} not exists", id);
                            assert_eq!(store.get_with_id(&id).unwrap().value, OFFSET + OFFSET2 + j);

                            let id2 = ResourceId::Counter(OFFSET2 + j);
                            assert!(store.exists(&id2), "Resource with id {:?} not exists", id2);
                            assert_eq!(store.get_with_id(&id2).unwrap().value, OFFSET2 + j);
                        }
                    }
                });
            }
        });
        assert_eq!(counter.load(Ordering::Relaxed), THREAD_COUNT + THREAD_COUNT + 1);
        // no change in resources
    }

    log::info!("bake with gc");
    {
        assert_eq!(counter.load(Ordering::Relaxed), THREAD_COUNT + THREAD_COUNT + 1); // no change in resources
        resources.bake::<ThreadedTest>(true);
        assert_eq!(counter.load(Ordering::Relaxed), 0); // all resources are released
    }

    log::info!("check bake with gc");
    {
        assert_eq!(counter.load(Ordering::Relaxed), 0); // no change in resources
        let res = resources.sync();
        rayon::scope(|s| {
            for _ in 0..THREAD_COUNT {
                s.spawn({
                    let res = &res;
                    move |_| {
                        let store = res.get_store::<ThreadedTest>().unwrap();

                        let id = ResourceId::Counter(1);
                        assert!(!store.exists(&id));

                        for j in 0..THREAD_COUNT {
                            let id = ResourceId::Counter(OFFSET + j);
                            assert!(!store.exists(&id));
                        }
                    }
                });
            }
        });
        assert_eq!(counter.load(Ordering::Relaxed), 0); // all resources are gone
    }
}
