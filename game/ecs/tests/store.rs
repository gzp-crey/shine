use shine_ecs::core::store::{Data, FromKey, Store};
use std::sync::Arc;
use std::{env, fmt, mem, thread};

fn init_logger() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();
}

fn single_threaded_test() {
    assert!(
        env::args().any(|a| a == "--test-threads=1")
            || env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "0".to_string()) == "1",
        "Force single threaded test execution. Command line: --test-threads=1, Env: RUST_TEST_THREADS=2"
    );
}

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

    fn on_load(&mut self, _load_response: Option<()>) -> Option<()> {
        None
    }
}

impl FromKey for TestData {
    fn from_key(k: &TestDataId) -> TestData {
        Self::new(format!("id: {}", k.0))
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
    init_logger();

    let store = Store::<TestData>::new(2);
    let r0;
    let r1;

    log::debug!("request 0,1");
    {
        let mut store = store.try_read().unwrap();
        assert!(store.named_get_blocking(&TestDataId(0)) == None);

        r0 = store.named_get_or_add_blocking(&TestDataId(0));
        assert!(store[&r0].0 == format!("id: {}", 0));

        r1 = store.named_get_or_add_blocking(&TestDataId(1));
        assert!(store[&r1].0 == format!("id: {}", 1));
        let r11 = store.named_get_blocking(&TestDataId(1)).unwrap();
        assert!(store[&r11].0 == format!("id: {}", 1));
        assert!(r11 == r1);
        let r12 = store.named_get_or_add_blocking(&TestDataId(1));
        assert!(store[&r12].0 == format!("id: {}", 1));
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
        assert!(store[&r0].0 == format!("id: {}", 0));
        assert!(store.named_get_blocking(&TestDataId(0)).unwrap() == r0);
        assert!(store[&r1].0 == format!("id: {}", 1));
        assert!(store.named_get_blocking(&TestDataId(1)).unwrap() == r1);

        let r2 = store.named_get_or_add_blocking(&TestDataId(2));
        assert!(store[&r2].0 == format!("id: {}", 2));
    }

    log::debug!("drop 2");
    {
        let mut store = store.try_write().unwrap();
        store.finalize_requests();
        store.drain_unused();
    }

    {
        let store = store.try_read().unwrap();
        assert!(store.named_get_blocking(&TestDataId(2)) == None);

        assert!(store[&r0].0 == format!("id: {}", 0));
        assert!(store.named_get_blocking(&TestDataId(0)).unwrap() == r0);
        assert!(store[&r1].0 == format!("id: {}", 1));
        assert!(store.named_get_blocking(&TestDataId(1)).unwrap() == r1);

        mem::drop(r1);
        // check that store is not yet modified
        assert!(store[&store.named_get_blocking(&TestDataId(1)).unwrap()].0 == format!("id: {}", 1));
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
        assert!(store[&r0].0 == format!("id: {}", 0));
        assert!(store.named_get_blocking(&TestDataId(0)).unwrap() == r0);
        assert!(store.named_get_blocking(&TestDataId(1)) == None);
        assert!(store.named_get_blocking(&TestDataId(2)) == None);

        mem::drop(r0);
        // check that store is not modified yet
        assert!(store[&store.named_get_blocking(&TestDataId(0)).unwrap()].0 == format!("id: {}", 0));
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
    init_logger();
    single_threaded_test();

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
                assert!(store.named_get_blocking(&TestDataId(0)) == None);

                // request 1
                let r1 = store.named_get_or_add_blocking(&TestDataId(1));
                assert!(store[&r1].0 == format!("id: {}", 1));

                // request 100 + threadId
                let r100 = store.named_get_or_add_blocking(&TestDataId(100 + i));
                assert!(store[&r100].0 == format!("id: {}", 100 + i));

                for _ in 0..100 {
                    assert!(store[&r1].0 == format!("id: {}", 1));
                    assert!(store[&r100].0 == format!("id: {}", 100 + i));
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
                assert!(store.named_get_blocking(&TestDataId(0)) == None);

                // get 1
                let r1 = store.named_get_blocking(&TestDataId(1)).unwrap();
                assert!(store[&r1].0 == format!("id: {}", 1));

                // get 100 + threadId
                let r100 = store.named_get_blocking(&TestDataId(100 + i)).unwrap();
                assert!(store[&r100].0 == format!("id: {}", 100 + i));
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
                assert!(store.named_get_blocking(&TestDataId(0)) == None);

                // get 1
                assert!(store.named_get_blocking(&TestDataId(1)) == None);

                // get 100 + threadId
                assert!(store.named_get_blocking(&TestDataId(100 + i)) == None);
            }));
        }
        for t in tp.drain(..) {
            t.join().unwrap();
        }
    }
}

#[test]
fn check_lock() {
    init_logger();
    single_threaded_test();

    use std::mem;
    use std::panic;

    panic::set_hook(Box::new(|_info| { /*println!("panic: {:?}", _info);*/ }));

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
