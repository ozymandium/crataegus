use axum::{
    body::to_bytes,
    body::Body,
    http::Request,
    response::Response,
    routing::{get, post},
    Router,
};
use clap::Parser;
use tokio::net::TcpListener;

use crataegus::gps_logger;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[clap(short, long, default_value = "8162")]
    port: u16,
}

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // Build our application with some routes
    let app = Router::new()
        .route("/", get(handle_request).post(handle_request))
        .fallback(handle_request);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Handler function for all requests
async fn handle_request(request: Request<Body>) -> Response<Body> {
    println!("Request received: {:?}", request);

    // Read the full body
    let content_length = request
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let body_bytes = to_bytes(request.into_body(), content_length).await.unwrap();
    let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

    if let Ok(body) = serde_json::from_str::<gps_logger::Body>(&body_str) {
        println!("Parsed body: {:?}", body);
    } else {
        println!("Failed to parse body:\n{}", body_str);
    }

    Response::new(Body::from("Request received"))
}
