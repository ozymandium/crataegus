use axum::{
    body::Body,
    extract::Extension,
    extract::Query,
    extract::State,
    http::Request,
    middleware::{self, Next},
    response::Response,
    routing::post,
    Router,
};
use axum_auth::AuthBasic;
use axum_server::tls_rustls::RustlsConfig;
use color_eyre::eyre::{ensure, eyre, Result, WrapErr};
use log::{debug, info, warn};
use serde::Deserialize;

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use crate::db::Db;
use crate::gpslogger;
use crate::schema::LocationGen;

/// Configuration for the server
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Port to listen on  
    port: u16,
    /// Path to the TLS certificate
    cert: PathBuf,
    /// Path to the TLS private key
    key: PathBuf,
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
    pub fn new(config: Config, db: Arc<Db>) -> Result<Self> {
        let _ = rustls::crypto::ring::default_provider()
            .install_default() // returns a Result<(), Arc(CryptoProvider)>
            .map_err(|_| eyre!("Failed to install default ring provider"));
        Ok(Server { config, db })
    }

    pub async fn serve(self) -> Result<()> {
        // config checks
        ensure!(self.config.cert.exists(), "Certificate file does not exist");
        ensure!(self.config.key.exists(), "Key file does not exist");

        let server = Arc::new(self);
        let protected_routes = Router::new()
            .route("/gpslogger", post(Self::handle_gpslogger))
            .layer(middleware::from_fn_with_state(server.clone(), Self::auth));
        let router = Router::new()
            .merge(protected_routes)
            .fallback(Self::handle_fallback)
            .with_state(server.clone());
        let rustls_config =
            RustlsConfig::from_pem_file(server.config.cert.clone(), server.config.key.clone())
                .await
                .wrap_err("Failed to load TLS config")?;

        let addr = SocketAddr::from(([0, 0, 0, 0], server.config.port));
        info!("Listening on {}", addr);

        axum_server::bind_rustls(addr, rustls_config)
            .serve(router.into_make_service())
            .await
            .wrap_err("Failed to start server")?;

        Ok(()) // reached after sever is stopped
    }

    /// Middleware layer to check for HTTP basic auth, used for user auth
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

    async fn handle_gpslogger(
        State(server): State<Arc<Server>>,
        Extension(AuthenticatedUser { username }): Extension<AuthenticatedUser>,
        Query(payload): Query<gpslogger::http::Payload>, // auto extracts query params from url
    ) -> Response<Body> {
        debug!("gpslogger url payload: {:?}", payload);
        server
            .db
            .location_insert(&LocationGen::to_location(&payload, &username))
            .await
            .unwrap();
        Response::new(Body::from("Request received"))
    }

    async fn handle_fallback(request: Request<Body>) -> Response<Body> {
        warn!("Fallback handler triggered. Request:\n{:#?}", request);
        Response::builder()
            .status(404)
            .body(Body::from("Not found"))
            .unwrap()
    }
}
