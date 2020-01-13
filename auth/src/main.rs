use actix_web::{middleware, App, HttpServer};
use data_encoding::BASE64;
use std::env;

mod auth;
mod config;

use auth::AuthService;
use shine_core::session::IdentityCookie;

/// Example of a main function of a actix server supporting oauth.
pub fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("auth", log::LevelFilter::Trace)
        .filter_module("mio", log::LevelFilter::Info)
        .filter_module("hyper", log::LevelFilter::Info)
        .filter_module("rustls", log::LevelFilter::Info)
        .filter_module("want", log::LevelFilter::Info)
        .init();

    let mut sys = actix_rt::System::new("Auth");

    let service_config = config::Config::new().expect("Service configuration failed");
    log::info!("{:#?}", service_config);
    let cookie_user_id_secret = BASE64
        .decode(service_config.cookie_session_secret.as_bytes())
        .expect("Failed to parse secret for user_id");

    let auth = AuthService::create(&mut sys, &service_config.auth).expect("Auth service creation failed");

    let _ = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(IdentityCookie::middleware(&cookie_user_id_secret))
            .configure(|cfg| auth.configure(cfg))
    })
    .workers(service_config.worker_count)
    .bind(service_config.get_bind_address())
    .expect("Server start failed")
    .run();

    log::info!("starting service on {}", service_config.get_bind_address());
    let _ = sys.run();
}
