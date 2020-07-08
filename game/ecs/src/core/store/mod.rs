mod store;
pub use store::*;
mod async_loader;
pub use async_loader::*;

pub fn no_load<D: Data>(page_size: usize) -> Store<D, ()> {
    Store::new(page_size)
}

pub fn async_load<D>(page_size: usize) -> Store<D, AsyncLoadContext<D>>
where
    D: Load<LoadContext = AsyncLoadContext<D>> + for<'l> OnLoad<'l>,
{
    use futures::channel::mpsc;

    let (request_sender, request_receiver) = mpsc::unbounded();
    let (response_sender, response_receiver) = mpsc::unbounded();

    let load_context = AsyncLoadContext {
        request_sender,
        response_sender: response_sender.clone(),
        response_receiver,
    };

    let load_worker = AsyncLoadWorker {
        request_receiver,
        response_sender,
    };
    load_worker.start();

    Store::new_with_load(page_size, load_context)
}
