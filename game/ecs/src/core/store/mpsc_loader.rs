use crate::core::store::{Data, Load, OnLoad, LoadToken};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::any::{Any, TypeId};

pub struct MPSCRequest<D>
where
    D: for<'l> OnLoad<'l>,
{
    request_sender: UnboundedSender<(<D as Load>::LoadRequest, LoadToken<D>)>,
    response_sender: UnboundedSender<(<D as Load>::LoadResponse, LoadToken<D>)>,
    response_receiver: UnboundedReceiver<(<D as Load>::LoadResponse, LoadToken<D>)>,
}

pub struct MPSCLoader<D>
where
    D: for<'l> OnLoad<'l>,
{
    request_receiver: UnboundedReceiver<(<D as Load>::LoadRequest, LoadToken<D>)>,
    response_sender: UnboundedSender<(<D as Load>::LoadResponse, LoadToken<D>)>,
    //data_loader: Box<dyn DataLoader<D>>,
}

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