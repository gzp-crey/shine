#![feature(drain_filter)]
#![feature(async_closure)]
#![feature(get_mut_unchecked)]
#![feature(type_name_of_val)]
#![feature(min_const_generics)]
#![allow(clippy::module_inception)]
#![allow(clippy::match_like_matches_macro)]

pub mod core;
pub mod resources;
pub mod scheduler;
pub mod utils;
pub mod world;

pub use hecs;
