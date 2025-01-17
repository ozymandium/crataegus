use std::path::PathBuf;

use clap::Parser;
use log::info;

use crataegus::server::Server;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Port to listen on for GpsLogger messages
    #[clap(short, long, env, default_value = "8162")]
    port: u16,
    /// Path to the sqlite database file
    #[clap(short, long, env, default_value = "~/.local/cretaegus.sqlite", value_hint = clap::ValueHint::FilePath)]
    db: PathBuf,
}

#[tokio::main]
async fn main() {
    color_eyre::install().unwrap();
    // Initialize logging
    env_logger::init();

    // Parse command line arguments.
    let args = Args::parse();
    info!("Starting Crataegus with args:\n{:#?}", args);

    let server = Server::new(args.port);
    server.serve().await;

    info!("Crataegus has stopped");
}
