use futures::{channel::oneshot, Future};

pub struct TaskCanceled;

/// Send task for asynchronous execution. The status can be polled
/// for completion withou blocking in a sync function.
/// Iternally a oneshot channel is used to trigger completion.
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

    pub fn try_get(&mut self) -> Result<Option<T>, TaskCanceled> {
        match self.receiver.try_recv() {
            Err(_) => Err(TaskCanceled),
            Ok(res) => Ok(res),
        }
    }
}
