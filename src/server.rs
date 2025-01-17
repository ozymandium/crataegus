use axum::{
    body::to_bytes, body::Body, extract::State, http::Request, response::Response, routing::post,
    Router,
};
use color_eyre::eyre::{eyre, Result};
use log::{debug, error, info, warn};
use serde::Deserialize;
use tokio::net::TcpListener;

use std::path::PathBuf;
use std::sync::Arc;

use crate::db::{Config as DbConfig, Db, Entry};
use crate::gpslogger;

#[derive(Debug, Deserialize)]
pub struct HttpConfig {
    port: u16,
}

/// Configuration for the server, obtained from main.rs::Args
#[derive(Debug, Deserialize)]
pub struct Config {
    http: HttpConfig,
    db: DbConfig,
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

/// The server struct
pub struct Server {
    config: HttpConfig,
    db: Db,
}

impl Server {
    pub async fn new(config: Config) -> Self {
        Self {
            config: config.http,
            db: Db::new(config.db).await,
        }
    }

    pub async fn serve(self) {
        let server = Arc::new(self);
        let router = Router::new()
            .route("/gpslogger", post(Self::handle_gpslogger))
            .fallback(Self::handle_fallback)
            .with_state(server.clone());

        let addr = format!("0.0.0.0:{}", server.config.port);
        info!("Listening on {}", addr);
        let listener = TcpListener::bind(&addr).await.unwrap();

        axum::serve(listener, router).await.unwrap();
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
