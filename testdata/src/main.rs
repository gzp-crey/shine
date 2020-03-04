mod auth;
mod config;

use self::config::Config;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let config = Config::new()?;

    auth::populate_roles(&config).await?;
    auth::populate_users(&config).await?;
    Ok(())
}
