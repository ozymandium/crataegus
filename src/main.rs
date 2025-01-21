use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use color_eyre::eyre::{eyre, Result};
use inquire::{Password, Text};
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

    #[clap(subcommand)]
    cmd: Cmd,
}

/// Configuration for the server, obtained from main.rs::Args
#[derive(Debug, Deserialize)]
pub struct Config {
    http: ServerConfig,
    db: DbConfig,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Start the HTTP server.
    Serve,
    /// Add a user to the database.
    Useradd,
    /// Import locations into the database.
    Import {
        #[clap(subcommand)]
        source: ImportSource,
    }
}

#[derive(Subcommand, Debug)]
enum ImportSource {
    /// Recursively search for EXIF GPSInfo in the given directory, importing it into the database.
    ExifDir {
        /// The directory to search.
        path: PathBuf,
    },
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

async fn serve(config: Config) -> Result<()> {
    info!("Starting Crataegus server");
    let db = Arc::new(Db::new(config.db).await?);
    let server = Server::new(config.http, db);
    server.serve().await?;
    Ok(())
}

async fn useradd(config: Config) -> Result<()> {
    println!("Adding a user to the database");
    let db = Arc::new(Db::new(config.db).await?);
    println!("Connected to the database. Enter the user information:");
    let username = Text::new("Username").prompt()?;
    let password = Password::new("Password").prompt()?;
    db.user_insert(&username, &password).await?;
    println!("User added successfully");
    Ok(())
}

async fn import_exif_dir(config: Config, path: PathBuf) -> Result<()> {
    use crataegus::exif::Finder;
    println!("Importing locations from directory: {}", path.display());
    let db = Arc::new(Db::new(config.db).await?);
    let finder = Finder::new(&path);
    let mut count = 0; 
    for location in finder {
        //db.location_insert(location).await?;
        count += 1;  
    }
    println!("Imported {} locations", count);  
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();
    env_logger::init();

    let args = Args::parse();
    info!("{:#?}", args);

    let config = Config::load(&args.config)?;
    info!("{:#?}", config);

    match args.cmd {
        Cmd::Serve => serve(config).await?,
        Cmd::Useradd => useradd(config).await?,
        Cmd::Import { source } => match source {
            ImportSource::ExifDir { path } => {
                import_exif_dir(config, path).await?;
            }
        },
    }

    Ok(())
}
