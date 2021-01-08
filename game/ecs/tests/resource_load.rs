use shine_ecs::resources::{
    ResourceHandle, ResourceId, ResourceLoadRequester, ResourceLoadResponder, ResourceLoader, Resources,
};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

mod utils;

/// Test resource data
#[derive(Debug)]
struct TestData {
    id: ResourceId,
    response_count: usize,
    text: String,
}

impl TestData {
    fn build(context: &ResourceLoadRequester<Self, String>, handle: ResourceHandle<Self>, id: &ResourceId) -> TestData {
        log::trace!("Creating [{:?}]", id);
        context.send_request(handle, "build".to_owned());
        TestData {
            id: id.clone(),
            response_count: 0,
            text: "!".to_owned(),
        }
    }

    async fn on_load(
        cnt: &Arc<AtomicUsize>,
        responder: &ResourceLoadResponder<Self, String>,
        handle: ResourceHandle<Self>,
        request: String,
    ) {
        log::trace!("on_load [{:?}]: {:?}", handle, request);
        cnt.fetch_add(1, Ordering::Relaxed);
        thread::sleep(Duration::from_micros(50)); // emulate an active wait
        responder.send_response(handle, format!("l({})", request));
    }

    fn on_load_response(
        this: &mut Self,
        context: &ResourceLoadRequester<Self, String>,
        handle: &ResourceHandle<Self>,
        response: String,
    ) {
        log::debug!("on_load_response [{:?}], {}", handle, response);
        this.response_count += 1;
        this.text = format!("{}, {}", this.text, response);
        context.send_request(handle.clone(), format!("r({})", this.text));
    }
}

#[tokio::test(threaded_scheduler)]
async fn simple() {
    utils::init_logger();

    let mut resources = Resources::default();
    let load_count = Arc::new(AtomicUsize::new(0));
    resources
        .register(ResourceLoader::new(
            TestData::build,
            load_count.clone(),
            TestData::on_load,
            TestData::on_load_response,
        ))
        .unwrap();

    {
        log::debug!("Create a resource");
        let id = {
            let store = resources.get_store::<TestData>().unwrap();
            store.get_handle(&ResourceId::from_tag(&"test").unwrap()).unwrap()
        };

        let mut i = 0;
        loop {
            log::debug!("Process loop {}", i);
            i += 1;
            assert!(
                i < 100,
                "Most certainly request-response is not working, as iteration limit was reached"
            );

            resources.bake::<TestData>(true);

            let response_count = {
                let store = resources.get_store::<TestData>().unwrap();
                let item = store.at(&id);
                log::trace!("loop: {}", item.text);
                item.response_count
            };

            let ld_count = load_count.load(Ordering::Relaxed);

            // we cannot have more response as load
            log::debug!("counters: loop:{} load:{}, response: {}", i, ld_count, response_count);
            assert!(ld_count >= response_count);

            if response_count > 3 {
                let store = resources.get_store::<TestData>().unwrap();
                let item = store.at(&id);
                assert!(item
                    .text
                    .starts_with("!, l(build), l(r(!, l(build))), l(r(!, l(build), l(r(!, l(build)))))"));
                break;
            } else {
                tokio::time::delay_for(Duration::from_micros(10)).await;
            }
        }
    }

    log::debug!("Clearing resources");
    resources.bake::<TestData>(true);
}
