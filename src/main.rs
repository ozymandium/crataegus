use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use color_eyre::eyre::{eyre, Result};
use log::info;
use serde::Deserialize;

use crataegus::db::{Config as DbConfig, Db};
use crataegus::server::{Config as ServerConfig, Server};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to the TOML configuration file.
    #[clap(short, long, env = "CRATAEGUS_CONFIG", default_value = "~/.config/crataegus.toml", value_hint = clap::ValueHint::FilePath)]
    config: PathBuf,
}

/// Configuration for the server, obtained from main.rs::Args
#[derive(Debug, Deserialize)]
pub struct Config {
    http: ServerConfig,
    db: DbConfig,
}

/// Implementation of the Config struct
impl Config {
    /// Load the configuration from a TOML file
    ///
    /// # Arguments
    /// * `path`: path to the TOML file
    ///
    /// # Returns
    /// The configuration struct
    pub fn load(path: &PathBuf) -> Result<Config> {
        if !path.exists() {
            return Err(eyre!("Config file does not exist: {}", path.display()));
        }
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();
    env_logger::init();

    let args = Args::parse();
    info!("Starting Crataegus with args:\n{:#?}", args);

    let config = Config::load(&args.config)?;

    let db = Arc::new(Db::new(config.db).await);

    let server = Server::new(config.http, db);
    server.serve().await?;

    info!("Crataegus has stopped");
    Ok(())
}
