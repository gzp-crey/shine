use shine_ecs::core::store::{self, AsyncLoadContext, AsyncLoader, Data, FromKey, LoadToken, OnLoad, OnBake};
use std::pin::Pin;
use std::sync::Arc;
use std::{fmt, mem, thread};

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

impl<'b> OnBake<'b> for TestData {
    type BakeContext = ();
    
    fn on_bake(&mut self, _bake_context:&mut ()) -> bool {
        self.0 += "baked";
        true
    }
}

impl OnLoad for TestData {
    type LoadRequest = String;
    type LoadResponse = String;
    type LoadContext = AsyncLoadContext<Self>;

    fn on_load_request(&self, load_context: &mut AsyncLoadContext<Self>, load_token: LoadToken<TestData>) {
        load_context.request(load_token, self.0.clone());
    }

    fn on_load_response(
        &mut self,
        load_context: &mut AsyncLoadContext<Self>,
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
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<String>>>> {
        Box::pin(async move { Some(format!("loaded - {}", request)) })
    }
}

#[tokio::test(threaded_scheduler)]
async fn simple() {
    utils::init_logger();

    let mut store = store::async_load(2, TestDataLoader);

    {
        let _id = {
            let mut store = store.try_read().unwrap();
            store.get_or_load(&"test".to_owned())
        };

        store.finalize_requests_with_bake(());
    }

    store.drain_unused();
}
