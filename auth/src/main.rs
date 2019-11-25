use actix_web::{middleware, App, HttpServer};
use base64;
use std::env;

mod auth;
mod config;
mod session;

use auth::AuthService;

/// Example of a main function of a actix server supporting oauth.
pub fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("shine_auth", log::LevelFilter::Trace)
        .init();

    let mut sys = actix::System::new("Auth");

    let service_config = config::Config::new().expect("Service configuration failed");
    log::info!("{:#?}", service_config);
    let user_id_secret = base64::decode(&service_config.user_id_secret).expect("Failed to parse secret for user_id");

    let auth = AuthService::create(&mut sys, &service_config.auth).expect("Auth service creation failed");

    let _ = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(session::UserId::cookie_session(&user_id_secret))
            .configure(|cfg| auth.configure(cfg))
    })
    .workers(service_config.worker_count)
    .bind(service_config.get_bind_address())
    .expect("Server start failed")
    .start();

    log::info!("starting service on {}", service_config.get_bind_address());
    let _ = sys.run();
}
