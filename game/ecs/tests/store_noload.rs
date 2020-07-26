use shine_ecs::core::store::{self, Data, FromKey};
use std::sync::Arc;
use std::{fmt, mem, thread};

mod utils;

/// Resource id for test data
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct TestDataId(u32);

impl fmt::Display for TestDataId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Test resource data
struct TestData(String);

impl Data for TestData {
    type Key = TestDataId;
}

impl FromKey for TestData {
    fn from_key(key: &TestDataId) -> TestData {
        Self::new(format!("id: {}", key.0))
    }
}

impl TestData {
    fn new(s: String) -> TestData {
        log::trace!("creating '{}'", s);
        TestData(s)
    }
}

impl Drop for TestData {
    fn drop(&mut self) {
        log::trace!("dropping '{}'", self.0);
    }
}

#[test]
fn simple_single_threaded() {
    utils::init_logger();

    let mut store = store::no_load::<TestData>(2);
    let r0;
    let r1;

    log::debug!("request 0,1");
    {
        let store = store.try_read().unwrap();
        assert!(store.try_get(&TestDataId(0)) == None);

        //r0 = store.get_or_load(&TestDataId(0)); // shall not compile as TestData is not OnLoad
        r0 = store.get_or_add(&TestDataId(0));

        assert!(store.at(&r0).0 == format!("id: {}", 0));

        r1 = store.get_or_add(&TestDataId(1));
        assert!(store.at(&r1).0 == format!("id: {}", 1));
        let r11 = store.try_get(&TestDataId(1)).unwrap();
        assert!(store.at(&r11).0 == format!("id: {}", 1));
        assert!(r11 == r1);
        let r12 = store.get_or_add(&TestDataId(1));
        assert!(store.at(&r12).0 == format!("id: {}", 1));
        assert!(r12 == r1);
    }

    log::debug!("request process");
    store.finalize_requests();

    log::debug!("check 0,1, request 2");
    {
        let store = store.try_read().unwrap();
        assert!(store.at(&r0).0 == format!("id: {}", 0));
        assert!(store.try_get(&TestDataId(0)).unwrap() == r0);
        assert!(store.at(&r1).0 == format!("id: {}", 1));
        assert!(store.try_get(&TestDataId(1)).unwrap() == r1);

        let r2 = store.get_or_add(&TestDataId(2));
        assert!(store.at(&r2).0 == format!("id: {}", 2));
    }

    log::debug!("drop 2");
    store.finalize_requests();
    store.drain_unused();

    {
        let store = store.try_read().unwrap();
        assert!(store.try_get(&TestDataId(2)) == None);

        assert!(store.at(&r0).0 == format!("id: {}", 0));
        assert!(store.try_get(&TestDataId(0)).unwrap() == r0);
        assert!(store.at(&r1).0 == format!("id: {}", 1));
        assert!(store.try_get(&TestDataId(1)).unwrap() == r1);

        mem::drop(r1);
        // check that store is not yet modified
        assert!(store.at(&store.try_get(&TestDataId(1)).unwrap()).0 == format!("id: {}", 1));
        //info!("{:?}", r1);
    }

    log::debug!("drop 1");
    store.finalize_requests();
    store.drain_unused();

    {
        let store = store.try_read().unwrap();
        assert!(store.at(&r0).0 == format!("id: {}", 0));
        assert!(store.try_get(&TestDataId(0)).unwrap() == r0);
        assert!(store.try_get(&TestDataId(1)) == None);
        assert!(store.try_get(&TestDataId(2)) == None);

        mem::drop(r0);
        // check that store is not modified yet
        assert!(store.at(&store.try_get(&TestDataId(0)).unwrap()).0 == format!("id: {}", 0));
    }

    log::debug!("drop 0");
    store.finalize_requests();
    store.drain_unused();
}

#[test]
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

#[test]
fn check_lock() {
    utils::init_logger();
    utils::single_threaded_test();

    use std::mem;
    use std::panic;

    panic::set_hook(Box::new(|_info| { /*println!("panic: {:?}", _info);*/ }));

    {
        let store = store::no_load::<TestData>(2);
        assert!(panic::catch_unwind(|| {
            let w = store.try_read().unwrap();
            let r = store.try_read().unwrap();
            drop(r);
            drop(w);
        })
        .is_ok());
        mem::forget(store);
    }

    {
        let store = store::no_load::<TestData>(2);
        assert!(panic::catch_unwind(|| {
            let w = store.try_write().unwrap();
            let r = store.try_read().unwrap();
            drop(r);
            drop(w);
        })
        .is_err());
        mem::forget(store);
    }

    {
        let store = store::no_load::<TestData>(2);
        assert!(panic::catch_unwind(|| {
            let r = store.try_read().unwrap();
            let w = store.try_write().unwrap();
            drop(w);
            drop(r);
        })
        .is_err());
        mem::forget(store);
    }

    {
        let store = store::no_load::<TestData>(2);
        assert!(panic::catch_unwind(|| {
            let w1 = store.try_write().unwrap();
            let w2 = store.try_write().unwrap();
            drop(w2);
            drop(w1);
        })
        .is_err());
        mem::forget(store);
    }

    let _ = panic::take_hook();
}
