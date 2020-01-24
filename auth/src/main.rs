use actix_web::{middleware, App, HttpServer};
use std::env;

mod auth;
mod config;

use auth::AuthService;

/// Example of a main function of a actix server supporting oauth.
pub fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::Builder::from_default_env()
        //.filter_level(log::LevelFilter::Info)
        .filter_level(log::LevelFilter::Trace)
        .filter_module("shine_auth", log::LevelFilter::Trace)
        .filter_module("shine_core", log::LevelFilter::Trace)
        .filter_module("mio", log::LevelFilter::Info)
        .filter_module("hyper", log::LevelFilter::Info)
        .filter_module("rustls", log::LevelFilter::Info)
        .filter_module("want", log::LevelFilter::Info)
        .init();

    let mut sys = actix_rt::System::new("Auth");

    let service_config = config::Config::new().expect("Service configuration failed");
    log::info!("{:#?}", service_config);

    let auth = AuthService::create(&mut sys, &service_config.auth).expect("Auth service creation failed");

    let _ = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .configure(|cfg| auth.configure(cfg))
    })
    .workers(service_config.worker_count)
    .bind(service_config.get_bind_address())
    .expect("Server start failed")
    .run();

    log::info!("starting service on {}", service_config.get_bind_address());
    let _ = sys.run();
}
