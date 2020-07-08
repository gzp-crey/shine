use crate::core::store::{Data, Load, LoadToken, OnLoad};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::any::{Any, TypeId};

pub struct AsyncLoadContext<D>
where
    D: for<'l> OnLoad<'l> + Load<LoadContext = Self>,
{
    pub(crate) request_sender: UnboundedSender<(LoadToken<D>, <D as Load>::LoadRequest)>,
    pub(crate) response_sender: UnboundedSender<(LoadToken<D>, <D as Load>::LoadResponse)>,
    pub(crate) response_receiver: UnboundedReceiver<(LoadToken<D>, <D as Load>::LoadResponse)>,
}

impl<D> AsyncLoadContext<D>
where
    D: for<'l> OnLoad<'l> + Load<LoadContext = Self>,
{
    pub fn request(&mut self, load_token: LoadToken<D>, request: <D as Load>::LoadRequest) {
        log::debug!("Request loading for [{:?}]", load_token);
        if let Err(err) = self.request_sender.unbounded_send((load_token, request)) {
            log::warn!("Failed to send request {:?}: {:?}", TypeId::of::<D>(), err);
        }
    }
}

pub struct AsyncLoadWorker<D>
where
    D: for<'l> OnLoad<'l>,
{
    pub(crate) request_receiver: UnboundedReceiver<(LoadToken<D>, <D as Load>::LoadRequest)>,
    pub(crate) response_sender: UnboundedSender<(LoadToken<D>, <D as Load>::LoadResponse)>,
    //data_loader: Box<dyn DataLoader<D>>,
}

//unsafe impl<D: Data> Send for AsyncLoadWorker<D> {}

impl<D: Data> AsyncLoadWorker<D>
where
    D: for<'l> OnLoad<'l>,
{
    async fn handle_one(&mut self) -> bool {
        let (load_token, data) = match self.request_receiver.next().await {
            Some((load_token, data)) => (load_token, data),
            None => {
                log::info!("Loader requests failed {:?}", TypeId::of::<D>());
                return false;
            }
        };

        if load_token.is_canceled() {
            return true;
        }
        /*let output = match self.data_loader.load(data, load_token.clone()).await {
            Some(output) => output,
            None => return true,
        };*/
        if load_token.is_canceled() {
            return true;
        }

        /* match self.response_sender.send((load_token, output)).await {
            Ok(_) => true,
            Err(err) => {
                log::info!("Loader response failed {:?}: {:?}", TypeId::of::<D>(), err);
                false
            }
        }*/
        true
    }

    async fn run(&mut self) {
        while self.handle_one().await {}
    }

    #[cfg(feature = "native")]
    pub fn start(mut self) {
        use tokio::{runtime::Handle, task};
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

/*
pub struct MPSCLoader<D>
where
    D: for<'l> OnLoad<'l>,
{
    request_receiver: UnboundedReceiver<(<D as Load>::LoadRequest, LoadToken<D>)>,
    response_sender: UnboundedSender<(<D as Load>::LoadResponse, LoadToken<D>)>,
    //data_loader: Box<dyn DataLoader<D>>,
}*/

/*impl<D> MPSCLoader<D>
where
    D: for<'l> OnLoad<'l>,
{
    async fn handle_one(&mut self) -> bool {
        let (data, load_token) = match self.load_request_receiver.next().await {
            Some((data, load_token)) => (data, load_token),
            None => {
                log::info!("Loader requests failed {:?}", TypeId::of::<D>());
                return false;
            }
        };

        if cancellation_token.is_canceled() {
            return true;
        }
        let output = match self.data_loader.load(data, cancellation_token.clone()).await {
            Some(output) => output,
            None => return true,
        };
        if cancellation_token.is_canceled() {
            return true;
        }

        match self.load_response_sender.send((output, cancellation_token)).await {
            Ok(_) => true,
            Err(err) => {
                log::info!("Loader response failed {:?}: {:?}", TypeId::of::<D>(), err);
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
*/
