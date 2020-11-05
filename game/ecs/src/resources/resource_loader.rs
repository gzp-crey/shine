use crate::resources::{Resource, ResourceHandle};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    SinkExt, StreamExt,
};
use std::future::Future;

/// Manage resource load request-responses, storage side
pub struct ResourceLoader<T, Request, Response>
where
    T: Resource,
    Request: Send,
    Response: Send,
{
    pub(crate) request_sender: UnboundedSender<(ResourceHandle<T>, Request)>,
    pub(crate) response_sender: UnboundedSender<(ResourceHandle<T>, Response)>,
    pub(crate) response_receiver: UnboundedReceiver<(ResourceHandle<T>, Response)>,
}

impl<T, Request, Response> ResourceLoader<T, Request, Response>
where
    T: Resource,
    Request: Send,
    Response: Send,
{
    pub fn request(&mut self, handle: ResourceHandle<T>, request: Request) {
        log::debug!("Request loading for {:?}", handle);
        if let Err(err) = self.request_sender.unbounded_send((handle, request)) {
            log::warn!("Failed to send request: {:?}", err);
        }
    }

    /*pub fn responder(&self) -> AsyncLoadResponder<D> {
        AsyncLoadResponder {
            response_sender: self.response_sender.clone(),
        }
    }*/

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
/*
/// Trait to create the loading future.
pub trait AsyncLoader<T, Request, Response>: 'static + Send + Sync
where T : Resource,
    Request : Send,
    Response: Send
{
    fn load<'a>(
        &'a mut self,
        load_token: LoadToken<D>,
        request: D::LoadRequest,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<D::LoadResponse>>>>;
}
*/
/*
/// Wrapper to send load responses.
pub struct AsyncLoadResponder<D>
where
    D: OnLoad<LoadHandler = AsyncLoadHandler<D>>,
{
    pub(crate) response_sender: UnboundedSender<(LoadToken<D>, D::LoadResponse)>,
}

impl<D> AsyncLoadResponder<D>
where
    D: OnLoad<LoadHandler = AsyncLoadHandler<D>>,
{
    pub fn send_response(&self, load_token: LoadToken<D>, response: D::LoadResponse) {
        log::trace!("[{:?}] Sending load response", load_token);
        if let Err(err) = self.response_sender.unbounded_send((load_token, response)) {
            log::info!("Failed to notify store: {:?}", err);
        }
    }
}
*/
/// Handle resource loading request, loading side.
pub struct ResourceLoadWorker<T, Request, Response, Load, Fut>
where
    T: Resource,
    Request: 'static + Send,
    Response: 'static + Send,
    Fut: 'static + Future<Output = Option<Response>>,
    Load: 'static + Fn(&ResourceHandle<T>, Request) -> Fut + Send,
{
    pub(crate) request_receiver: UnboundedReceiver<(ResourceHandle<T>, Request)>,
    pub(crate) response_sender: UnboundedSender<(ResourceHandle<T>, Response)>,
    pub(crate) load: Load,
}

impl<T, Request, Response, Load, Fut> ResourceLoadWorker<T, Request, Response, Load, Fut>
where
    T: Resource,
    Request: 'static + Send,
    Response: 'static + Send,
    Fut: 'static + Future<Output = Option<Response>>,
    Load: 'static + Fn(&ResourceHandle<T>, Request) -> Fut + Send,
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
