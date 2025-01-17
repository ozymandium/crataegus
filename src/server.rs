use axum::{
    body::to_bytes, body::Body, extract::State, http::Request, response::Response, routing::post,
    Router,
};
use color_eyre::eyre::{eyre, Result, WrapErr};
use log::{debug, error, info, warn};
use serde::Deserialize;
use tokio::net::TcpListener;

use std::path::PathBuf;
use std::sync::Arc;

use crate::db::{Db, Entry};
use crate::gpslogger;

/// Configuration for the server
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Port to listen on  
    port: u16,
}

/// The server struct
pub struct Server {
    /// Configuration for the server
    config: Config,
    /// Database connection
    db: Arc<Db>,
}

impl Server {
    pub fn new(config: Config, db: Arc<Db>) -> Self {
        Server { config, db }
    }

    pub async fn serve(self) -> Result<()> {
        let server = Arc::new(self);
        let router = Router::new()
            .route("/gpslogger", post(Self::handle_gpslogger))
            .fallback(Self::handle_fallback)
            .with_state(server.clone());

        let addr = format!("0.0.0.0:{}", server.config.port);
        info!("Listening on {}", addr);
        let listener = TcpListener::bind(&addr)
            .await
            .wrap_err("Failed to bind to address")?;

        axum::serve(listener, router)
            .await
            .wrap_err("Failed to serve")?;
        Ok(()) // This is unreachable
    }

    async fn get_body_string(request: Request<Body>) -> String {
        let content_length = request
            .headers()
            .get("content-length")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let body_bytes = to_bytes(request.into_body(), content_length).await.unwrap();
        String::from_utf8(body_bytes.to_vec()).unwrap()
    }

    async fn handle_gpslogger(
        State(server): State<Arc<Server>>,
        request: Request<Body>,
    ) -> Response<Body> {
        debug!("Request received: {:?}", request);
        todo!("Add user, from http basic auth");
        let body = Self::get_body_string(request).await;
        let payload = match gpslogger::Payload::from_http_body(&body) {
            Ok(payload) => payload,
            Err(e) => {
                error!("Failed to parse body: {}", e);
                todo!("Ntfy");
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
        todo!("Ntfy");
        Response::new(Body::from("Fallback response"))
    }
}
