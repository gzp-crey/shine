#![feature(drain_filter)]
#![feature(async_closure)]
#![feature(get_mut_unchecked)]
#![feature(type_name_of_val)]
#![feature(min_const_generics)]
#![allow(clippy::module_inception)]

pub mod core;
pub mod resources;
pub mod scheduler;
pub mod utils;

pub use hecs;
