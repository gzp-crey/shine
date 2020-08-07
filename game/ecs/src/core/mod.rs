pub mod arena;
pub mod async_task;
pub mod observer;
pub mod spscstate;
pub mod store;

pub trait WasmSend {}

impl<T> WasmSend for T {}
