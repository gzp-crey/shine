use crate::resources::{Resource, ResourceBakeContext, ResourceConfig, ResourceHandle, ResourceId};
use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    SinkExt, StreamExt,
};
use std::{any::Any, future::Future};

/// Context to request (async) resource load
pub struct ResourceLoadContext<T: Resource, RQ> {
    request_sender: UnboundedSender<(ResourceHandle<T>, RQ)>,
}

impl<T: Resource, RQ> ResourceLoadContext<T, RQ> {
    pub fn send_request(&self, handle: ResourceHandle<T>, rq: RQ) {
        log::trace!("[{:?}] Sending load request", handle);
        if let Err(err) = self.request_sender.unbounded_send((handle, rq)) {
            log::info!("Failed to notify load worker: {:?}", err);
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
    build: Box<dyn Fn(&ResourceLoadContext<T, RQ>, ResourceHandle<T>, &ResourceId) -> T>,
    response: Box<dyn Fn(&ResourceLoadContext<T, RQ>, &ResourceHandle<T>, &mut T, RP)>,
}

impl<T, RQ, RP> ResourceLoader<T, RQ, RP>
where
    T: Resource,
    RQ: 'static + Send,
    RP: 'static + Send,
{
    pub fn new<Build, Load, LoadFut, Response>(build: Build, load: Load, response: Response) -> Self
    where
        Build: 'static + Fn(&ResourceLoadContext<T, RQ>, ResourceHandle<T>, &ResourceId) -> T,
        LoadFut: 'static + Future<Output = Option<RP>>,
        Load: 'static + Send + Fn(&ResourceHandle<T>, RQ) -> LoadFut,
        Response: 'static + Fn(&ResourceLoadContext<T, RQ>, &ResourceHandle<T>, &mut T, RP),
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
        let request_context = ResourceLoadContext {
            request_sender: self.request_sender.clone(),
        };
        (self.build)(&request_context, handle, id)
    }

    fn post_bake(&mut self, context: &mut ResourceBakeContext<'_, Self::Resource>) {
        while let Some((handle, rp)) = self.next_response() {
            log::trace!("[{:?}] Received load response", handle);

            let request_context = ResourceLoadContext {
                request_sender: self.request_sender.clone(),
            };
            context.process_by_handle(&handle, {
                let request_context = &request_context;
                let response = &self.response;
                move |handle, resource| {
                    log::trace!("[{:?}] On load response", handle);
                    (response)(request_context, &handle, resource, rp);
                }
            });
        }
    }

    fn auto_gc(&self) -> bool {
        true
    }
}

/// Handle resource loading request, loading side.
struct ResourceLoadWorker<T, RQ, RP, Load, LoadFut>
where
    T: Resource,
    RQ: 'static + Send,
    RP: 'static + Send,
    LoadFut: 'static + Future<Output = Option<RP>>,
    Load: 'static + Send + Fn(&ResourceHandle<T>, RQ) -> LoadFut,
{
    request_receiver: UnboundedReceiver<(ResourceHandle<T>, RQ)>,
    response_sender: UnboundedSender<(ResourceHandle<T>, RP)>,
    load: Load,
}

impl<T, RQ, RP, Load, LoadFut> ResourceLoadWorker<T, RQ, RP, Load, LoadFut>
where
    T: Resource,
    RQ: 'static + Send,
    RP: 'static + Send,
    LoadFut: 'static + Future<Output = Option<RP>>,
    Load: 'static + Send + Fn(&ResourceHandle<T>, RQ) -> LoadFut,
{
    async fn handle_one(&mut self) -> bool {
        let (handle, rq) = match self.request_receiver.next().await {
            Some((handle, rq)) => (handle, rq),
            None => {
                log::warn!("Failed to get next load request, channel closed");
                return false;
            }
        };

        log::trace!("Loading {:?}", handle);
        if !handle.is_alive() {
            return true;
        }
        let response = match (self.load)(&handle, rq).await {
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
