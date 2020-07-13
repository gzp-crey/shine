use shine_ecs::core::store::{self, AsyncLoadHandler, AsyncLoader, Data, FromKey, LoadToken, OnLoad, OnLoading};
use std::pin::Pin;
use std::time::Duration;

mod utils;

/// Test resource data
#[derive(Debug)]
struct TestData {
    text: String,
    request_count: u32,
    response_count: u32,
}

impl TestData {
    fn new(text: String) -> TestData {
        log::trace!("creating '{}'", text);
        TestData {
            text,
            request_count: 0,
            response_count: 0,
        }
    }
}

impl Data for TestData {
    type Key = String;
}

impl FromKey for TestData {
    fn from_key(key: &String) -> TestData {
        Self::new(format!("from_key({})", key))
    }
}

impl<'b> OnLoading<'b> for TestData {
    type LoadingContext = ();
}

impl OnLoad for TestData {
    type LoadRequest = String;
    type LoadResponse = String;
    type LoadHandler = AsyncLoadHandler<Self>;

    fn on_load_request(&mut self, load_handler: &mut AsyncLoadHandler<Self>, load_token: LoadToken<TestData>) {
        log::debug!("on_load_request pre: {:?}", self);
        self.request_count += 1;
        load_handler.request(load_token, self.text.clone());
        log::debug!("on_load_request post: {:?}", self);
    }

    fn on_load_response(
        &mut self,
        load_handler: &mut AsyncLoadHandler<Self>,
        _loading_context: &mut (),
        load_token: LoadToken<TestData>,
        load_response: String,
    ) {
        log::debug!("on_load_response pre: {:?}, {:?}", self, load_response);
        self.response_count += 1;
        self.text = format!("on_load_response({}, {})", self.text, load_response);
        self.request_count += 1;
        load_handler.request(load_token, self.text.clone());
        log::debug!("on_load_response post: {:?}", self);
    }
}

struct TestDataLoader;

impl AsyncLoader<TestData> for TestDataLoader {
    fn load<'a>(
        &'a mut self,
        _load_token: LoadToken<TestData>,
        request: String,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<String>>>> {
        Box::pin(async move {
            let response = format!("load({})", request);
            log::debug!("async loading {} -> {}", request, response);
            Some(response)
        })
    }
}

#[tokio::test(threaded_scheduler)]
async fn simple() {
    utils::init_logger();

    let mut store = store::async_load(2, TestDataLoader);

    {
        log::debug!("Creating item");
        let id = {
            let mut store = store.try_read().unwrap();
            store.get_or_load(&"test".to_owned())
        };

        for i in 0..3 {
            tokio::time::delay_for(Duration::from_micros(100)).await;
            log::debug!("Load {}", i);
            store.load_and_finalize_requests(());

            {
                let store = store.try_read().unwrap();
                let item = store.at(&id);
                assert!(item.request_count >= item.response_count);
            }
        }
    }

    log::debug!("Clearing store");
    store.drain_unused();
}
