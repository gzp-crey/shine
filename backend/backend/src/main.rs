use actix_web::{middleware, App, HttpServer};
use shine_auth::AuthService;
use shine_gamestate::GameStateService;
use shine_web::WebService;
use std::env;

mod config;

/// Example of a main function of a actix server supporting oauth.
pub fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        //.filter_level(log::LevelFilter::Trace)
        .filter_module("shine_auth", log::LevelFilter::Trace)
        .filter_module("shine_web", log::LevelFilter::Trace)
        .filter_module("shine_gamestate", log::LevelFilter::Trace)
        .filter_module("shine_core", log::LevelFilter::Debug)
        .filter_module("mio", log::LevelFilter::Info)
        .filter_module("hyper", log::LevelFilter::Info)
        .filter_module("rustls", log::LevelFilter::Info)
        .filter_module("want", log::LevelFilter::Info)
        .init();

    let mut sys = actix_rt::System::new("Auth");

    let service_config = config::Config::new().expect("Service configuration failed");
    log::info!("{:#?}", service_config);

    let auth = AuthService::create(&mut sys, &service_config.auth, "auth").expect("Auth service creation failed");
    let web = WebService::create(&mut sys, &service_config.web, "web").expect("Web service creation failed");
    let gamestate = GameStateService::create(&mut sys, &service_config.gamestate, "gamestate")
        .expect("GameState service creation failed");

    let _ = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .configure(|cfg| web.configure(cfg))
            .configure(|cfg| auth.configure(cfg))
            .configure(|cfg| gamestate.configure(cfg))
    })
    .workers(service_config.worker_count)
    .bind(service_config.get_bind_address())
    .expect("Server start failed")
    .run();

    log::info!("starting service on {}", service_config.get_bind_address());
    let _ = sys.run();
}
