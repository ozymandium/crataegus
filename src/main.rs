use std::path::PathBuf;

use axum::{body::to_bytes, body::Body, http::Request, response::Response, routing::post, Router};
use clap::Parser;
use log::{debug, error, info, warn};
use tokio::net::TcpListener;

use crataegus::gpslogger;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Port to listen on for GpsLogger messages
    #[clap(short, long, default_value = "8162", env = "CRATAEGUS_PORT")]
    port: u16,
    /// Path to the sqlite database file
    #[clap(short, long, default_value = "~/.local/cretaegus.sqlite", env = "CRATAEGUS_DB", value_hint = clap::ValueHint::FilePath)]
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

    // Build our application with some routes
    let router = Router::new()
        .route("/gpslogger", post(handle_gpslogger))
        .fallback(handle_fallback);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
    axum::serve(listener, router).await.unwrap();
}

// Handler function for all requests
async fn handle_gpslogger(request: Request<Body>) -> Response<Body> {
    debug!("Request received: {:?}", request);

    // Read the full body
    let content_length = request
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let body_bytes = to_bytes(request.into_body(), content_length).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    // log an error if the body is not parseable. otherwise, parse payload.
    let _payload = match gpslogger::Payload::from_http_body(&body_str) {
        Ok(_payload) => _payload,
        Err(e) => {
            error!("Failed to parse body: {}", e);
            return Response::new(Body::from("Failed to parse body"));
        }
    };

    Response::new(Body::from("Request received"))
}

async fn handle_fallback(request: Request<Body>) -> Response<Body> {
    warn!("Fallback handler triggered. Request:\n{:#?}", request);
    Response::new(Body::from("Fallback response"))
}
