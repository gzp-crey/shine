use shine_ecs::resources::{ResourceHandle, ResourceId, ResourceLoadContext, ResourceLoader, Resources};
use std::{thread, time::Duration};

mod utils;

/// Test resource data
#[derive(Debug)]
struct TestData {
    id: ResourceId,
    response_count: u32,
    text: String,
}

impl TestData {
    fn build(context: &ResourceLoadContext<Self, String>, handle: ResourceHandle<Self>, id: &ResourceId) -> TestData {
        log::trace!("Creating [{:?}]", id);
        context.send_request(handle, "build".to_string());
        TestData {
            id: id.clone(),
            response_count: 0,
            text: "!".to_string(),
        }
    }

    async fn on_load(handle: ResourceHandle<Self>, request: String) -> Option<String> {
        log::debug!("on_load [{:?}]: {:?}", handle, request);
        thread::sleep(Duration::from_micros(50)); // emulate an active wait
        Some(format!("l({})", request))
    }

    fn on_load_response(
        this: &mut Self,
        context: &ResourceLoadContext<Self, String>,
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
    resources.register(ResourceLoader::new(
        TestData::build,
        TestData::on_load,
        TestData::on_load_response,
    ));

    {
        log::debug!("Create a resource");
        let id = {
            let store = resources.get_store::<TestData>().unwrap();
            store.get_handle(&ResourceId::from_tag(&"test").unwrap()).unwrap()
        };

        let mut i = 0;
        let mut response_count = 0;
        loop {
            log::debug!("Process loop {} - {}", i, response_count);
            i += 1;
            assert!(
                i < 100,
                "Most certainly request-response is not working, as iteration limit was reached"
            );

            resources.bake::<TestData>(true);

            response_count = {
                let store = resources.get_store::<TestData>().unwrap();
                let item = store.at(&id);
                log::debug!("loop: {}", item.text);
                item.response_count
            };

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
