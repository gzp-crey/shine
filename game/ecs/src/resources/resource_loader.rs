use crate::resources::{Resource, ResourceBakeContext, ResourceConfig, ResourceHandle, ResourceId};
use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    StreamExt,
};
use std::{any::Any, future::Future};

/// Wrapper to solve lifetime issues.
/// see: https://users.rust-lang.org/t/issue-with-fnonce-and-async-function-with-a-reference-type-argument/51959?u=gzp
pub trait FnResourceLoad<'a, CTX, T, RQ, RP>: 'static + Send
where
    T: Resource,
    CTX: 'static + Send,
    RP: 'static + Send,
    RQ: 'static + Send,
{
    type Fut: 'a + Future<Output = ()>;

    fn call(
        &self,
        responder: ResourceLoadResponder<T, RP>,
        context: &'a CTX,
        handle: ResourceHandle<T>,
        request: RQ,
    ) -> Self::Fut;
}

impl<'a, CTX, T, RP, RQ, LoadFut, FnLoad> FnResourceLoad<'a, CTX, T, RQ, RP> for FnLoad
where
    T: Resource,
    CTX: 'static + Send,
    RP: 'static + Send,
    RQ: 'static + Send,
    LoadFut: 'a + Future<Output = ()>,
    FnLoad: 'static + Send + Fn(ResourceLoadResponder<T, RP>, &'a CTX, ResourceHandle<T>, RQ) -> LoadFut,
{
    type Fut = LoadFut;

    fn call(
        &self,
        responder: ResourceLoadResponder<T, RP>,
        context: &'a CTX,
        handle: ResourceHandle<T>,
        request: RQ,
    ) -> Self::Fut {
        (self)(responder, context, handle, request)
    }
}

/// Request a resource to be loaded
pub struct ResourceLoadRequester<T: Resource, RQ> {
    request_sender: UnboundedSender<(ResourceHandle<T>, RQ)>,
}

impl<T: Resource, RQ> ResourceLoadRequester<T, RQ> {
    pub fn send_request(&self, handle: ResourceHandle<T>, rq: RQ) {
        log::trace!("[{:?}] Sending load request", handle);
        if let Err(err) = self.request_sender.unbounded_send((handle, rq)) {
            log::info!("Failed to notify load worker: {:?}", err);
        }
    }
}

/// Respond to a resource to load request
pub struct ResourceLoadResponder<T: Resource, RP> {
    response_sender: UnboundedSender<(ResourceHandle<T>, RP)>,
}

impl<T: Resource, RP> ResourceLoadResponder<T, RP> {
    pub fn send_response(&self, handle: ResourceHandle<T>, rp: RP) {
        log::trace!("[{:?}] Sending load response", handle);
        if let Err(err) = self.response_sender.unbounded_send((handle, rp)) {
            log::info!("Failed to send response: {:?}", err);
        }
    }
}

/// Manage resource load request-responses, storage side
#[allow(clippy::type_complexity)]
pub struct ResourceLoader<T, RQ, RP>
where
    T: Resource,
    RQ: 'static + Send,
    RP: 'static + Send,
{
    request_sender: UnboundedSender<(ResourceHandle<T>, RQ)>,
    response_receiver: UnboundedReceiver<(ResourceHandle<T>, RP)>,
    build: Box<dyn Fn(&ResourceLoadRequester<T, RQ>, ResourceHandle<T>, &ResourceId) -> T>,
    response: Box<dyn Fn(&mut T, &ResourceLoadRequester<T, RQ>, &ResourceHandle<T>, RP)>,
}

impl<T, RQ, RP> ResourceLoader<T, RQ, RP>
where
    T: Resource,
    RQ: 'static + Send,
    RP: 'static + Send,
{
    pub fn new<CTX, FnBuild, FnLoad, FnResponse>(
        build: FnBuild,
        context: CTX,
        load: FnLoad,
        response: FnResponse,
    ) -> Self
    where
        CTX: 'static + Send + Sync,
        FnBuild: 'static + Fn(&ResourceLoadRequester<T, RQ>, ResourceHandle<T>, &ResourceId) -> T,
        for<'a> FnLoad: FnResourceLoad<'a, CTX, T, RQ, RP>,
        FnResponse: 'static + Fn(&mut T, &ResourceLoadRequester<T, RQ>, &ResourceHandle<T>, RP),
    {
        let (request_sender, request_receiver) = mpsc::unbounded();
        let (response_sender, response_receiver) = mpsc::unbounded();

        ResourceLoadWorker {
            request_receiver,
            response_sender,
            load,
            context,
        }
        .start();

        Self {
            request_sender,
            response_receiver,
            build: Box::new(build),
            response: Box::new(response),
        }
    }

    pub fn request(&mut self, handle: ResourceHandle<T>, rq: RQ) {
        log::debug!("Request loading for {:?}", handle);
        if let Err(err) = self.request_sender.unbounded_send((handle, rq)) {
            log::warn!("Failed to send request: {:?}", err);
        }
    }

    fn next_response(&mut self) -> Option<(ResourceHandle<T>, RP)> {
        match self.response_receiver.try_next() {
            Ok(Some(response)) => Some(response),
            Ok(None) => {
                log::warn!("Failed to get next load response, channel closed");
                None
            }
            Err(_) => None,
        }
    }
}

impl<T, RQ, RP> ResourceConfig for ResourceLoader<T, RQ, RP>
where
    T: Resource,
    RQ: 'static + Send,
    RP: 'static + Send,
{
    type Resource = T;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn auto_build(&self) -> bool {
        true
    }

    fn build(&self, handle: ResourceHandle<T>, id: &ResourceId) -> Self::Resource {
        let request_context = ResourceLoadRequester {
            request_sender: self.request_sender.clone(),
        };
        (self.build)(&request_context, handle, id)
    }

    fn post_bake(&mut self, context: &mut ResourceBakeContext<'_, Self::Resource>) {
        while let Some((handle, rp)) = self.next_response() {
            log::trace!("[{:?}] Received load response", handle);

            let request_context = ResourceLoadRequester {
                request_sender: self.request_sender.clone(),
            };
            context.process_by_handle(&handle, {
                let request_context = &request_context;
                let response = &self.response;
                move |handle, resource| {
                    log::trace!("[{:?}] On load response", handle);
                    (response)(resource, request_context, &handle, rp);
                }
            });
        }
    }
}

/// Handle resource loading request, loading side.
struct ResourceLoadWorker<CTX, T, RQ, RP, FnLoad>
where
    CTX: 'static + Send + Sync,
    T: Resource,
    RQ: 'static + Send,
    RP: 'static + Send,
    for<'a> FnLoad: FnResourceLoad<'a, CTX, T, RQ, RP>,
{
    request_receiver: UnboundedReceiver<(ResourceHandle<T>, RQ)>,
    response_sender: UnboundedSender<(ResourceHandle<T>, RP)>,
    load: FnLoad,
    context: CTX,
}

impl<CTX, T, RQ, RP, FnLoad> ResourceLoadWorker<CTX, T, RQ, RP, FnLoad>
where
    CTX: 'static + Send + Sync,
    T: Resource,
    RQ: 'static + Send,
    RP: 'static + Send,
    for<'a> FnLoad: FnResourceLoad<'a, CTX, T, RQ, RP>,
{
    pub async fn run(&mut self) {
        while let Some((handle, rq)) = self.request_receiver.next().await {
            log::trace!("Loading {:?}", handle);
            if !handle.is_alive() {
                continue;
            }
            let responder = ResourceLoadResponder {
                response_sender: self.response_sender.clone(),
            };
            self.load.call(responder, &self.context, handle.clone(), rq).await;

            // when channels are closed, we are done
            // response_sender is checked explicitly, but request_receiver is checked in the loop
            if self.response_sender.is_closed() {
                break;
            }
        }
    }

    #[cfg(feature = "native")]
    pub fn start(mut self) {
        use tokio::{runtime::Handle, task};
        log::debug!("Starting async loader");
        task::spawn_blocking(move || {
            Handle::current().block_on(task::LocalSet::default().run_until(async move {
                task::spawn_local(async move {
                    self.run().await;
                })
                .await
                .unwrap()
            }))
        });
    }

    #[cfg(feature = "wasm")]
    pub fn start(mut self) {
        use wasm_bindgen_futures::spawn_local;
        spawn_local(async move {
            self.run().await;
        });
    }
}
