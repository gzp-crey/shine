use shine_ecs::core::store::{self, AsyncLoadContext, Data, FromKey, Load, LoadToken, OnLoad, Store, AsyncLoader};
use std::sync::Arc;
use std::{fmt, mem, thread};
use std::pin::Pin;

mod utils;

/// Test resource data
struct TestData(String);

impl TestData {
    fn new(s: String) -> TestData {
        log::trace!("creating '{}'", s);
        TestData(s)
    }
}

impl Data for TestData {
    type Key = String;
}

impl FromKey for TestData {
    fn from_key(key: &String) -> TestData {
        Self::new(format!("id: {}", key))
    }
}

impl Load for TestData {
    type LoadRequest = String;
    type LoadResponse = String;
    type LoadContext = AsyncLoadContext<Self>;

    fn on_load_request(&self, load_context: &mut AsyncLoadContext<Self>, load_token: LoadToken<TestData>) {
        load_context.request(load_token, self.0.clone());
    }
}

impl<'l> OnLoad<'l> for TestData {
    type UpdateContext = ();

    fn on_load_response(
        &mut self,
        load_context: &mut AsyncLoadContext<Self>,
        _update_context: (),
        load_token: LoadToken<TestData>,
        load_response: String,
    ) {
        self.0 = load_response;
        load_context.request(load_token, self.0.clone());
    }
}

struct TestDataLoader;

impl AsyncLoader<TestData> for TestDataLoader {
    fn load<'a>(
        &'a mut self,
        load_token: LoadToken<TestData>,
        request: String,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<String>>>>
    {
        move async || {
            Some(format!("loaded - {}", request))
        }
    }
}

#[tokio::test(threaded_scheduler)]
async fn simple() {
    utils::init_logger();
    
    let store = store::async_load::<TestData, TestDataLoader>(2, TestDataLoader);
}
