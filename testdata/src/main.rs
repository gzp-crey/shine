mod auth;
mod config;

use self::config::Config;
use env_logger;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let config = Config::new()?;

    auth::populate_roles(&config).await?;
    auth::populate_users(&config).await?;
    Ok(())
}
