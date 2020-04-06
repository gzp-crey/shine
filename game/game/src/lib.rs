mod error;
pub use self::error::*;
mod gamerender;
pub use self::gamerender::*;

pub mod input;
pub mod render;
pub mod tasks;

//reexport wgpu for easier project maintenance
#[cfg(feature = "wasm")]
pub use wasm_wgpu as wgpu;

#[cfg(feature = "native")]
pub use native_wgpu as wgpu;
