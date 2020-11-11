use crate::resources::{Resource, ResourceBakeContext, ResourceConfig, ResourceHandle, ResourceId};
use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    SinkExt, StreamExt,
};
use std::{any::Any, future::Future};

/// Context to request (async) resource load
pub struct ResourceLoadRequester<T: Resource, Request> {
    request_sender: UnboundedSender<(ResourceHandle<T>, Request)>,
}

impl<T: Resource, Request> ResourceLoadRequester<T, Request> {
    pub fn send_request(&self, handle: ResourceHandle<T>, request: Request) {
        log::trace!("[{:?}] Sending load request", handle);
        if let Err(err) = self.request_sender.unbounded_send((handle, request)) {
            log::info!("Failed to notify load worker: {:?}", err);
        }
    }
}

/// Context to respond to the load request with the result of loading
pub struct ResourceLoadResponder<T: Resource, Response> {
    response_sender: UnboundedSender<(ResourceHandle<T>, Response)>,
}

impl<T: Resource, Response> ResourceLoadResponder<T, Response> {
    pub fn send_response(&self, handle: ResourceHandle<T>, response: Response) {
        log::trace!("[{:?}] Sending load response", handle);
        if let Err(err) = self.response_sender.unbounded_send((handle, response)) {
            log::info!("Failed to notify store: {:?}", err);
        }
    }
}

/// Manage resource load request-responses, storage side
pub struct ResourceLoader<T, Request, Response>
where
    T: Resource,
    Request: 'static + Send,
    Response: 'static + Send,
{
    request_sender: UnboundedSender<(ResourceHandle<T>, Request)>,
    response_receiver: UnboundedReceiver<(ResourceHandle<T>, Response)>,
    build: Box<dyn Fn(ResourceLoadRequester<T, Request>, ResourceHandle<T>, &ResourceId) -> T>,
    response: Box<dyn Fn(ResourceLoadRequester<T, Request>, ResourceHandle<T>, &mut T, Response)>,
}

impl<T, Request, Response> ResourceLoader<T, Request, Response>
where
    T: Resource,
    Request: 'static + Send,
    Response: 'static + Send,
{
    pub fn new<FBuild, FLoad, FLoadFut, FResponse>(build: FBuild, load: FLoad, response: FResponse) -> Self
    where
        FBuild: 'static + Fn(ResourceLoadRequester<T, Request>, ResourceHandle<T>, &ResourceId) -> T,
        FLoadFut: 'static + Future<Output = Option<Response>>,
        FLoad: 'static + Send + Fn(&ResourceHandle<T>, Request) -> FLoadFut,
        FResponse: 'static + Fn(ResourceLoadRequester<T, Request>, ResourceHandle<T>, &mut T, Response),
    {
        let (request_sender, request_receiver) = mpsc::unbounded();
        let (response_sender, response_receiver) = mpsc::unbounded();

        ResourceLoadWorker {
            request_receiver,
            response_sender,
            load,
        }
        .start();

        Self {
            request_sender,
            response_receiver,
            build: Box::new(build),
            response: Box::new(response),
        }
    }

    pub fn request(&mut self, handle: ResourceHandle<T>, request: Request) {
        log::debug!("Request loading for {:?}", handle);
        if let Err(err) = self.request_sender.unbounded_send((handle, request)) {
            log::warn!("Failed to send request: {:?}", err);
        }
    }

    fn next_response(&mut self) -> Option<(ResourceHandle<T>, Response)> {
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

impl<T, Request, Response> ResourceConfig for ResourceLoader<T, Request, Response>
where
    T: Resource,
    Request: 'static + Send,
    Response: 'static + Send,
{
    type Resource = T;

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn auto_build(&self) -> bool {
        true
    }

    fn build(&self, handle: ResourceHandle<T>, id: &ResourceId) -> Self::Resource {
        (self.build)(
            ResourceLoadRequester {
                request_sender: self.request_sender.clone(),
            },
            handle,
            id,
        )
    }

    fn post_bake(&self, _context: &mut ResourceBakeContext<'_, Self::Resource>) {
        unimplemented!()
    }

    fn auto_gc(&self) -> bool {
        true
    }
}

/// Handle resource loading request, loading side.
struct ResourceLoadWorker<T, Request, Response, Load, LoadFut>
where
    T: Resource,
    Request: 'static + Send,
    Response: 'static + Send,
    LoadFut: 'static + Future<Output = Option<Response>>,
    Load: 'static + Send + Fn(&ResourceHandle<T>, Request) -> LoadFut,
{
    request_receiver: UnboundedReceiver<(ResourceHandle<T>, Request)>,
    response_sender: UnboundedSender<(ResourceHandle<T>, Response)>,
    load: Load,
}

impl<T, Request, Response, Load, LoadFut> ResourceLoadWorker<T, Request, Response, Load, LoadFut>
where
    T: Resource,
    Request: 'static + Send,
    Response: 'static + Send,
    LoadFut: 'static + Future<Output = Option<Response>>,
    Load: 'static + Send + Fn(&ResourceHandle<T>, Request) -> LoadFut,
{
    async fn handle_one(&mut self) -> bool {
        let (handle, request) = match self.request_receiver.next().await {
            Some((handle, request)) => (handle, request),
            None => {
                log::warn!("Failed to get next load request, channel closed");
                return false;
            }
        };

        log::trace!("Loading {:?}", handle);
        if !handle.is_alive() {
            return true;
        }
        let response = match (self.load)(&handle, request).await {
            Some(output) => output,
            None => return true,
        };
        if !handle.is_alive() {
            return true;
        }

        log::trace!("[{:?}] Sending load response", handle);
        match self.response_sender.send((handle, response)).await {
            Ok(_) => true,
            Err(err) => {
                log::info!("Loader response failed: {:?}", err);
                false
            }
        }
    }

    async fn run(&mut self) {
        while self.handle_one().await {}
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
