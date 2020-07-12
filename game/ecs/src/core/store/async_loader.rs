use crate::core::store::{Data, LoadGuard, LoadHandler, LoadToken, OnLoad};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::any::{Any, TypeId};
use std::pin::Pin;

pub struct AsyncLoadContext<D>
where
    D: OnLoad<LoadContext = Self>,
{
    pub(crate) request_sender: UnboundedSender<(LoadToken<D>, D::LoadRequest)>,
    pub(crate) response_sender: UnboundedSender<(LoadToken<D>, D::LoadResponse)>,
    pub(crate) response_receiver: UnboundedReceiver<(LoadToken<D>, D::LoadResponse)>,
}

impl<D> AsyncLoadContext<D>
where
    D: OnLoad<LoadContext = Self>,
{
    pub fn request(&mut self, load_token: LoadToken<D>, request: D::LoadRequest) {
        log::debug!("Request loading for [{:?}]", load_token);
        if let Err(err) = self.request_sender.unbounded_send((load_token, request)) {
            log::warn!("Failed to send request {:?}: {:?}", TypeId::of::<D>(), err);
        }
    }
}

impl<D> LoadHandler<D> for AsyncLoadContext<D>
where
    D: OnLoad<LoadContext = Self>,
{
    fn load<'l>(&mut self, store: LoadGuard<'l, D>) {
        unimplemented!()
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

//unsafe impl<D: Data> Send for AsyncLoadWorker<D> {}

impl<D: Data> AsyncLoadWorker<D>
where
    D: OnLoad,
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
        let output = match self.loader.load(load_token.clone(), data).await {
            Some(output) => output,
            None => return true,
        };
        if load_token.is_canceled() {
            return true;
        }

        match self.response_sender.send((load_token, output)).await {
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

/*
pub struct MPSCLoader<D>
where
    D: for<'l> OnLoad<'l>,
{
    request_receiver: UnboundedReceiver<(<D as Load>::LoadRequest, LoadToken<D>)>,
    response_sender: UnboundedSender<(Q, LoadToken<D>)>,
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
