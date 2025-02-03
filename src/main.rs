use std::io::Write;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use env_logger::{Builder as LogBuilder, Env as LogEnv};
use log::info;

use crataegus::cli::{backup, export, import, info, serve, useradd, Config, ImportFormat};
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
    Info {
        /// Optionally specify a username to get info for
        #[clap(short, long)]
        username: Option<String>,
    },
}

/// Configure the logging system with env_logger. Call this function at the beginning of main.
/// Disables timestamping, since systemd will add timestamps to the logs.
fn setup_logging() {
    // allows setting the RUST_LOG environment variable to control logging
    LogBuilder::from_env(LogEnv::default())
        .format(|buf, record| {
            let module_path = record.module_path().unwrap_or_default();
            let line = record.line().unwrap_or_default();
            let level_style = buf.default_level_style(record.level());
            writeln!(
                buf,
                "{level_style}[{}] {}:{}\n{}{level_style:#}",
                record.level(),
                module_path,
                line,
                record.args()
            )
        })
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    setup_logging();

    let args = Args::parse();
    info!("{:#?}", args);

    let config = Config::load(&args.config)?;
    info!("{:#?}", config);

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
        } => export(config, format, &path, &username, &start_str, &stop_str).await?,
        Cmd::Import {
            format,
            path,
            username,
        } => import(config, format, &path, &username).await?,
        Cmd::Info { username } => info(config, username.as_deref()).await?,
    }

    Ok(())
}
