use axum::{
    body::to_bytes,
    body::Body,
    extract::Extension,
    extract::State,
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::post,
    Router,
};
use axum_auth::AuthBasic;
use color_eyre::eyre::{Result, WrapErr};
use log::{debug, error, info, warn};
use serde::Deserialize;
use tokio::net::TcpListener;

use std::sync::Arc;

use crate::db::{Db, Location};
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

/// Struct to hold the authenticated user as an extension for protected routes
#[derive(Clone)]
struct AuthenticatedUser {
    username: String,
}

impl Server {
    pub fn new(config: Config, db: Arc<Db>) -> Self {
        Server { config, db }
    }

    pub async fn serve(self) -> Result<()> {
        let server = Arc::new(self);
        let protected_routes = Router::new()
            .route("/gpslogger", post(Self::handle_gpslogger))
            //.layer(middleware::from_fn(Self::auth)); // leave this line last
            .layer(middleware::from_fn_with_state(server.clone(), Self::auth));
        let router = Router::new()
            .merge(protected_routes)
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

    /// Middleware layer to check for HTTP basic auth
    async fn auth(
        State(server): State<Arc<Server>>,
        AuthBasic((username, password)): AuthBasic,
        mut request: Request<Body>,
        next: Next,
    ) -> Response<Body> {
        debug!("Authenticating user: {}", username);
        let good = server
            .db
            .user_check(&username, &password.unwrap_or_default())
            .await
            .unwrap();
        if !good {
            warn!("Failed to authenticate user: {}", username);
            return Response::builder().status(401).body(Body::empty()).unwrap();
        }
        // Add the authenticated user to the request extensions
        request
            .extensions_mut()
            .insert(AuthenticatedUser { username });
        next.run(request).await
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
        Extension(AuthenticatedUser { username }): Extension<AuthenticatedUser>,
        request: Request<Body>,
    ) -> Response<Body> {
        debug!("Request received: {:?}", request);
        let body = Self::get_body_string(request).await;
        let payload = match gpslogger::Payload::from_http_body(&body) {
            Ok(payload) => payload,
            Err(e) => {
                error!("Failed to parse body: {}", e);
                return Response::new(Body::from("Failed to parse body"));
            }
        };
        let location = Location {
            username: username,
            time: payload.timestamp,
            latitude: payload.lat,
            longitude: payload.lon,
            accuracy: payload.acc,
            altitude: payload.alt,
        };
        server.db.record(location).await.unwrap();
        Response::new(Body::from("Request received"))
    }

    async fn handle_fallback(request: Request<Body>) -> Response<Body> {
        warn!("Fallback handler triggered. Request:\n{:#?}", request);
        Response::new(Body::from("Fallback response"))
    }
}
