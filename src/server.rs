use axum::{
    body::to_bytes, body::Body, extract::State, http::Request, response::Response, routing::post,
    Router,
};
use log::{debug, error, info, warn};
use std::sync::Arc;
use tokio::net::TcpListener;

use crate::gpslogger;

pub struct Server {
    port: u16,
}

impl Server {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    pub async fn serve(self) {
        // Create a single Arc<Server> at startup
        let server = Arc::new(self);

        // Build our application with some routes
        let router = Router::new()
            .route("/gpslogger", post(Self::handle_gpslogger))
            .fallback(Self::handle_fallback)
            .with_state(server.clone());

        let addr = format!("0.0.0.0:{}", server.port);
        info!("Listening on {}", addr);
        let listener = TcpListener::bind(&addr).await.unwrap();

        axum::serve(listener, router).await.unwrap();
    }

    // Handler function for all requests
    async fn handle_gpslogger(
        State(server): State<Arc<Server>>,
        request: Request<Body>,
    ) -> Response<Body> {
        debug!("Request received: {:?}", request);
        let content_length = request
            .headers()
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let body_bytes = to_bytes(request.into_body(), content_length).await.unwrap();
        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

        let _payload = match gpslogger::Payload::from_http_body(&body_str) {
            Ok(_payload) => _payload,
            Err(e) => {
                error!("Failed to parse body: {}", e);
                return Response::new(Body::from("Failed to parse body"));
            }
        };

        Response::new(Body::from("Request received"))
    }

    async fn handle_fallback(
        State(server): State<Arc<Server>>,
        request: Request<Body>,
    ) -> Response<Body> {
        warn!("Fallback handler triggered. Request:\n{:#?}", request);
        Response::new(Body::from("Fallback response"))
    }
}
