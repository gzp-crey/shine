pub mod arena;
pub mod spscstate;
pub mod store;

pub trait WasmSend {}

impl<T> WasmSend for T {}
