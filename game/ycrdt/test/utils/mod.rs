#![allow(dead_code)]

use env_logger;
use std::env;

pub fn init_logger() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();
}