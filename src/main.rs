use std::path::PathBuf;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

use crataegus::cli::{backup, export, import, serve, useradd, Config, ImportFormat};
use crataegus::export::Format as ExportFormat;

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

    Ok(())
}
