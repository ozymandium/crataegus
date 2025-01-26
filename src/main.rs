use std::path::PathBuf;
use std::sync::Arc;

use chrono_english::parse_date_string;
use clap::{Parser, Subcommand, ValueEnum};
use color_eyre::eyre::{eyre, Result};
use futures::StreamExt;
use inquire::{Password, Text};
use log::info;
use serde::Deserialize;

use crataegus::db::{Config as DbConfig, Db};
use crataegus::export::{create_exporter, Format as ExportFormat};
use crataegus::gpslogger::csv::read_csv;
use crataegus::server::{Config as ServerConfig, Server};

/// Command line arguments
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
    https: ServerConfig,
    db: DbConfig,
}

/// Top level subcommands
#[derive(Subcommand, Debug)]
enum Cmd {
    /// Start the server
    Serve,
    /// Add a user to the database
    Useradd,
    /// Backup the database. May be called while server is running.
    Backup,
    /// Export the database to a file
    Export {
        // all these values are required
        #[arg(value_enum)]
        format: ExportFormat,

        #[clap(value_hint = clap::ValueHint::FilePath)]
        path: PathBuf,

        username: String,

        start_str: String,

        stop_str: String,
    },
    Import {
        /// The format of the file to import
        #[arg(value_enum)]
        format: ImportFormat,

        /// The path to the file to import
        #[clap(value_hint = clap::ValueHint::FilePath)]
        path: PathBuf,

        /// The username to associate with the imported data
        username: String,
    },
}

/// Types of supported imports
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ImportFormat {
    /// GPSLogger CSV format
    GpsLoggerCsv,
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
        let content = std::fs::read_to_string(path)
            .map_err(|e| eyre!("Failed to read config file: {}", e))?;
        let config: Config =
            toml::from_str(&content).map_err(|e| eyre!("Failed to parse config file: {}", e))?;
        Ok(config)
    }
}

async fn serve(config: Config) -> Result<()> {
    info!("Starting Crataegus server");
    let db = Arc::new(
        Db::new(config.db)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
    );
    let server =
        Server::new(config.https, db).map_err(|e| eyre!("Failed to create server: {}", e))?;
    server
        .serve()
        .await
        .map_err(|e| eyre!("Server failed: {}", e))?;
    Ok(())
}

async fn useradd(config: Config) -> Result<()> {
    println!("Adding a user to the database");
    let db = Arc::new(
        Db::new(config.db)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
    );
    println!("Connected to the database. Enter the user information:");
    let username = Text::new("Username").prompt()?;
    let password = Password::new("Password").prompt()?;
    db.user_insert(&username, &password)
        .await
        .map_err(|e| eyre!("Failed to add user: {}", e))?;
    println!("User added successfully");
    Ok(())
}

async fn backup(config: Config) -> Result<()> {
    println!("Backing up the database");
    let db = Arc::new(
        Db::new(config.db)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
    );
    db.backup()
        .await
        .map_err(|e| eyre!("Failed to backup database: {}", e))?;
    println!("Database backed up successfully");
    Ok(())
}

async fn export(
    config: Config,
    format: ExportFormat,
    path: PathBuf,
    username: String,
    start_str: String,
    stop_str: String,
) -> Result<()> {
    let now = chrono::offset::Local::now().fixed_offset();
    let start = parse_date_string(&start_str, now, chrono_english::Dialect::Us)
        .map_err(|_| eyre!("Failed to parse start date"))?;
    let stop = parse_date_string(&stop_str, now, chrono_english::Dialect::Us)
        .map_err(|_| eyre!("Failed to parse stop date"))?;
    println!(
        "Exporting\n  format: {:?}\n  path: {}\n  start: {}\n  stop: {}",
        format,
        path.display(),
        start,
        stop
    );
    let db = Arc::new(
        Db::new(config.db)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
    );
    let name = format!(
        "crataegus_export_{}_{}",
        start.to_rfc3339(),
        stop.to_rfc3339()
    );
    let mut exporter = create_exporter(format, &name, &path)
        .map_err(|e| eyre!("Failed to create exporter: {}", e))?;
    let mut location_stream = db
        .location_stream(&username, start.to_utc(), stop.to_utc())
        .await
        .map_err(|e| eyre!("Failed to get location stream: {}", e))?;
    let mut count = 0;
    while let Some(location) = location_stream.next().await {
        let location = location.map_err(|e| eyre!("A location in the stream failed: {}", e))?;
        exporter
            .write_location(&location)
            .map_err(|e| eyre!("Failed to write location: {}", e))?;
        count += 1;
    }
    exporter.finish()?;
    println!("Exported {} locations", count);
    Ok(())
}

async fn import(
    config: Config,
    format: ImportFormat,
    path: PathBuf,
    username: String,
) -> Result<()> {
    println!(
        "Importing\n  format: {:?}\n  path: {}",
        format,
        path.display()
    );
    let db = Arc::new(
        Db::new(config.db)
            .await
            .map_err(|e| eyre!("Failed to connect to database: {}", e))?,
    );
    let mut count = 0;
    match format {
        ImportFormat::GpsLoggerCsv => {
            let iter =
                read_csv(path, username).map_err(|e| eyre!("Failed to read CSV file: {}", e))?;
            for location in iter {
                let location = location.map_err(|e| eyre!("Failed to read location: {}", e))?;
                db.location_insert(location)
                    .await
                    .map_err(|e| eyre!("Failed to insert location: {}", e))?;
                count += 1;
            }
        }
    }
    println!("Imported {} locations", count);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();
    env_logger::init();

    let args = Args::parse();
    println!("{:#?}", args);

    let config = Config::load(&args.config)?;
    println!("{:#?}", config);

    match args.cmd {
        Cmd::Serve => serve(config).await?,
        Cmd::Useradd => useradd(config).await?,
        Cmd::Backup => backup(config).await?,
        Cmd::Export {
            format,
            path,
            username,
            start_str,
            stop_str,
        } => export(config, format, path, username, start_str, stop_str).await?,
        Cmd::Import {
            format,
            path,
            username,
        } => import(config, format, path, username).await?,
    }

    info!("Crataegus has stopped");
    Ok(())
}
