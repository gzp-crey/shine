use shine_ecs::core::store::{Data, FromKey, Store};
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
    type LoadRequest = ();
    type LoadResponse = ();
}

impl FromKey for TestData {
    fn from_key(key: &TestDataId) -> (TestData, Option<()>) {
        (Self::new(format!("id: {}", key.0)), None)
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

    let store = Store::<TestData>::new(2);
    let r0;
    let r1;

    log::debug!("request 0,1");
    {
        let mut store = store.try_read().unwrap();
        assert!(store.try_get_blocking(&TestDataId(0)) == None);

        r0 = store.get_or_add_blocking(&TestDataId(0));
        assert!(store.at(&r0).0 == format!("id: {}", 0));

        r1 = store.get_or_add_blocking(&TestDataId(1));
        assert!(store.at(&r1).0 == format!("id: {}", 1));
        let r11 = store.try_get_blocking(&TestDataId(1)).unwrap();
        assert!(store.at(&r11).0 == format!("id: {}", 1));
        assert!(r11 == r1);
        let r12 = store.get_or_add_blocking(&TestDataId(1));
        assert!(store.at(&r12).0 == format!("id: {}", 1));
        assert!(r12 == r1);
    }

    log::debug!("request process");
    {
        let mut store = store.try_write().unwrap();
        store.finalize_requests();
    }

    log::debug!("check 0,1, request 2");
    {
        let mut store = store.try_read().unwrap();
        assert!(store.at(&r0).0 == format!("id: {}", 0));
        assert!(store.try_get_blocking(&TestDataId(0)).unwrap() == r0);
        assert!(store.at(&r1).0 == format!("id: {}", 1));
        assert!(store.try_get_blocking(&TestDataId(1)).unwrap() == r1);

        let r2 = store.get_or_add_blocking(&TestDataId(2));
        assert!(store.at(&r2).0 == format!("id: {}", 2));
    }

    log::debug!("drop 2");
    {
        let mut store = store.try_write().unwrap();
        store.finalize_requests();
        store.drain_unused();
    }

    {
        let store = store.try_read().unwrap();
        assert!(store.try_get_blocking(&TestDataId(2)) == None);

        assert!(store.at(&r0).0 == format!("id: {}", 0));
        assert!(store.try_get_blocking(&TestDataId(0)).unwrap() == r0);
        assert!(store.at(&r1).0 == format!("id: {}", 1));
        assert!(store.try_get_blocking(&TestDataId(1)).unwrap() == r1);

        mem::drop(r1);
        // check that store is not yet modified
        assert!(store.at(&store.try_get_blocking(&TestDataId(1)).unwrap()).0 == format!("id: {}", 1));
        //info!("{:?}", r1);
    }

    log::debug!("drop 1");
    {
        let mut store = store.try_write().unwrap();
        store.finalize_requests();
        store.drain_unused();
    }

    {
        let store = store.try_read().unwrap();
        assert!(store.at(&r0).0 == format!("id: {}", 0));
        assert!(store.try_get_blocking(&TestDataId(0)).unwrap() == r0);
        assert!(store.try_get_blocking(&TestDataId(1)) == None);
        assert!(store.try_get_blocking(&TestDataId(2)) == None);

        mem::drop(r0);
        // check that store is not modified yet
        assert!(store.at(&store.try_get_blocking(&TestDataId(0)).unwrap()).0 == format!("id: {}", 0));
    }

    log::debug!("drop 0");
    {
        let mut store = store.try_write().unwrap();
        store.finalize_requests();
        store.drain_unused();
        assert!(store.is_empty());
    }
}

#[test]
fn simple_multi_threaded() {
    utils::init_logger();
    utils::single_threaded_test();

    let store = Store::<TestData>::new(2);
    let store = Arc::new(store);

    const ITER: u32 = 10;

    // request from multiple threads
    {
        let mut tp = vec![];
        for i in 0..ITER {
            let store = store.clone();
            tp.push(thread::spawn(move || {
                let mut store = store.try_read().unwrap();
                assert!(store.try_get_blocking(&TestDataId(0)) == None);

                // request 1
                let r1 = store.get_or_add_blocking(&TestDataId(1));
                assert!(store.at(&r1).0 == format!("id: {}", 1));

                // request 100 + threadId
                let r100 = store.get_or_add_blocking(&TestDataId(100 + i));
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
    {
        let mut store = store.try_write().unwrap();
        store.finalize_requests();
        // no drain
    }

    // check after process
    {
        let mut tp = vec![];
        for i in 0..ITER {
            let store = store.clone();
            tp.push(thread::spawn(move || {
                let store = store.try_read().unwrap();
                assert!(store.try_get_blocking(&TestDataId(0)) == None);

                // get 1
                let r1 = store.try_get_blocking(&TestDataId(1)).unwrap();
                assert!(store.at(&r1).0 == format!("id: {}", 1));

                // get 100 + threadId
                let r100 = store.try_get_blocking(&TestDataId(100 + i)).unwrap();
                assert!(store.at(&r100).0 == format!("id: {}", 100 + i));
            }));
        }
        for t in tp.drain(..) {
            t.join().unwrap();
        }
    }

    log::info!("drain");
    {
        let mut store = store.try_write().unwrap();
        store.finalize_requests();
        store.drain_unused();
        // no drain
    }

    // check after drain
    {
        let mut tp = vec![];
        for i in 0..ITER {
            let store = store.clone();
            tp.push(thread::spawn(move || {
                let store = store.try_read().unwrap();
                assert!(store.try_get_blocking(&TestDataId(0)) == None);

                // get 1
                assert!(store.try_get_blocking(&TestDataId(1)) == None);

                // get 100 + threadId
                assert!(store.try_get_blocking(&TestDataId(100 + i)) == None);
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
        let store = Store::<TestData>::new(2);
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
        let store = Store::<TestData>::new(2);
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
        let store = Store::<TestData>::new(2);
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
        let store = Store::<TestData>::new(2);
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
