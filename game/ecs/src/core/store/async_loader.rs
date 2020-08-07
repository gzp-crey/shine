use crate::core::store::{Data, LoadHandler, LoadToken, OnLoad};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    SinkExt, StreamExt,
};
use std::pin::Pin;

pub struct AsyncLoadHandler<D>
where
    D: OnLoad<LoadHandler = Self>,
{
    pub(crate) request_sender: UnboundedSender<(LoadToken<D>, D::LoadRequest)>,
    pub(crate) response_sender: UnboundedSender<(LoadToken<D>, D::LoadResponse)>,
    pub(crate) response_receiver: UnboundedReceiver<(LoadToken<D>, D::LoadResponse)>,
}

impl<D> AsyncLoadHandler<D>
where
    D: OnLoad<LoadHandler = Self>,
{
    pub fn request(&mut self, load_token: LoadToken<D>, request: D::LoadRequest) {
        log::debug!("Request loading for {:?}", load_token);
        if let Err(err) = self.request_sender.unbounded_send((load_token, request)) {
            log::warn!("Failed to send request: {:?}", err);
        }
    }

    pub fn send_response(&mut self, load_token: LoadToken<D>, response: D::LoadResponse) {
        log::trace!("[{:?}] Sending load response", load_token);
        if let Err(err) = self.response_sender.unbounded_send((load_token, response)) {
            log::info!("Failed to notify store: {:?}", err);
        }
    }
}

impl<D> LoadHandler<D> for AsyncLoadHandler<D>
where
    D: OnLoad<LoadHandler = Self>,
{
    fn next_response(&mut self) -> Option<(LoadToken<D>, D::LoadResponse)> {
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

pub trait AsyncLoader<D>: 'static + Send + Sync
where
    D: OnLoad,
{
    fn load<'a>(
        &'a mut self,
        load_token: LoadToken<D>,
        request: D::LoadRequest,
    ) -> Pin<Box<dyn 'a + std::future::Future<Output = Option<D::LoadResponse>>>>;
}

pub struct AsyncLoadWorker<D>
where
    D: OnLoad,
{
    pub(crate) request_receiver: UnboundedReceiver<(LoadToken<D>, D::LoadRequest)>,
    pub(crate) response_sender: UnboundedSender<(LoadToken<D>, D::LoadResponse)>,
    pub(crate) loader: Box<dyn AsyncLoader<D>>,
}

impl<D: Data> AsyncLoadWorker<D>
where
    D: OnLoad,
{
    async fn handle_one(&mut self) -> bool {
        let (load_token, data) = match self.request_receiver.next().await {
            Some((load_token, data)) => (load_token, data),
            None => {
                log::warn!("Failed to get next load request, channel closed");
                return false;
            }
        };

        log::trace!("Loading {:?}", load_token);
        if load_token.is_canceled() {
            return true;
        }
        let output = match self.loader.load(load_token.clone(), data).await {
            Some(output) => output,
            None => return true,
        };
        if load_token.is_canceled() {
            return true;
        }

        log::trace!("[{:?}] Sending load response", load_token);
        match self.response_sender.send((load_token, output)).await {
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
            Handle::current().block_on(task::LocalSet::new().run_until(async move {
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
