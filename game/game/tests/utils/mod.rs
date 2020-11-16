#![allow(dead_code)]

use env_logger;
use std::env;

pub fn init_logger() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .is_test(true)
        .try_init();
}

pub fn single_threaded_test() {
    assert!(
        env::args().any(|a| a == "--test-threads=1")
            || env::var("RUST_TEST_THREADS").unwrap_or_else(|_| "0".to_string()) == "1",
        "Force single threaded test execution. Command line: --test-threads=1, Env: RUST_TEST_THREADS=2"
    );
}
