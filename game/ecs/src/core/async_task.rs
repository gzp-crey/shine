use futures::{channel::oneshot, Future};

pub struct Canceled;

/// Send task into an async anvironment and poll for completion in non-async context using a oneshot channel
pub struct AsyncTask<T> {
    receiver: oneshot::Receiver<T>,
}

impl<T> AsyncTask<T>
where
    T: 'static + Send,
{
    pub fn start<F>(task: F) -> AsyncTask<T>
    where
        F: 'static + Future<Output = T> + Send,
    {
        let (sender, receiver) = oneshot::channel();

        let async_task = async move {
            let data = task.await;
            log::debug!("Sending async task");
            if sender.send(data).is_err() {
                log::warn!("Failed to send async task result");
            }
        };

        #[cfg(feature = "native")]
        {
            use tokio::{runtime::Handle, task};
            task::spawn_blocking(move || Handle::current().block_on(async_task));
        }

        #[cfg(feature = "wasm")]
        {
            use wasm_bindgen_futures::spawn_local;
            spawn_local(async_task);
        }

        AsyncTask { receiver }
    }

    pub fn try_get(&mut self) -> Result<Option<T>, Canceled> {
        match self.receiver.try_recv() {
            Err(_) => Err(Canceled),
            Ok(res) => Ok(res),
        }
    }
}
