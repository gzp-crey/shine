use actix_web::{middleware, App, HttpServer};
use std::env;

mod auth;
mod config;
mod session;

/// Example of a main function of a actix server supporting oauth.
pub fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .filter_module("shine_auth", log::LevelFilter::Trace)
        .init();

    let sys = actix::System::new("Auth");

    let service_config = config::Config::default();
    let secret = service_config.secret_user_id.clone();

    let _ = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(session::UserId::cookie_session(&secret))
            .configure(|cfg| {
                auth::configure_service(cfg);
            })
    })
    .workers(service_config.worker_count)
    .bind(service_config.get_bind_address())
    .expect("Server start failed")
    .start();

    log::info!("starting service on {}", service_config.get_bind_address());
    let _ = sys.run();
}
