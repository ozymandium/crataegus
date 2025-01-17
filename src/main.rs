use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::Result;
use log::info;

use crataegus::server::{Config, Server};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Path to the TOML configuration file.
    #[clap(short, long, env = "CRATAEGUS_CONFIG", default_value = "~/.config/crataegus.toml", value_hint = clap::ValueHint::FilePath)]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();
    env_logger::init();

    let args = Args::parse();
    info!("Starting Crataegus with args:\n{:#?}", args);

    let config = Config::load(&args.config)?;

    let server = Server::new(config).await;
    server.serve().await?;

    info!("Crataegus has stopped");
    Ok(())
}
