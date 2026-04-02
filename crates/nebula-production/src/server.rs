//! HTTP server setup with graceful shutdown
//!
//! Provides:
//! - Axum-based HTTP server
//! - Graceful shutdown handling
//! - Middleware integration (logging, metrics, CORS)
//! - Health and metrics endpoints

use crate::config::{ProductionConfig, ServerConfigInner};
use crate::health::{HealthChecker, HealthState};
use crate::metrics::{MetricsCollector, MetricsState};
use axum::routing::get;
use axum::Router;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{error, info};

/// Server configuration
#[derive(Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub shutdown_timeout_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            shutdown_timeout_secs: 30,
        }
    }
}

impl From<&ServerConfigInner> for ServerConfig {
    fn from(config: &ServerConfigInner) -> Self {
        Self {
            host: config.host.clone(),
            port: config.port,
            shutdown_timeout_secs: config.shutdown_timeout_secs,
        }
    }
}

/// HTTP server with graceful shutdown
pub struct Server {
    config: ServerConfig,
    app: Router,
    health_checker: Arc<HealthChecker>,
    metrics_collector: Arc<MetricsCollector>,
}

impl Server {
    /// Create a new server with the given configuration
    pub fn new(config: &ProductionConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let server_config = ServerConfig::from(&config.server);
        
        let health_checker = Arc::new(HealthChecker::new());
        let metrics_collector = Arc::new(MetricsCollector::new());

        // Build the app with middleware
        let app = Self::build_app(
            server_config.clone(),
            health_checker.clone(),
            metrics_collector.clone(),
        );

        Ok(Self {
            config: server_config,
            app,
            health_checker,
            metrics_collector,
        })
    }

    /// Build the Axum router with all middleware and routes
    fn build_app(
        _config: ServerConfig,
        health_checker: Arc<HealthChecker>,
        metrics_collector: Arc<MetricsCollector>,
    ) -> Router {
        // Create states
        let health_state = HealthState {
            checker: health_checker.clone(),
        };
        let metrics_state = MetricsState {
            collector: metrics_collector.clone(),
        };

        // Base routes
        let api_routes = Router::new()
            .route("/", get(root_handler))
            .route("/ping", get(ping_handler));

        // Health routes with state
        let health_router = Router::new()
            .route("/health/live", get(crate::health::liveness_handler))
            .route("/health/ready", get(crate::health::readiness_handler))
            .route("/health", get(crate::health::health_handler))
            .with_state(health_state);

        // Metrics routes with state
        let metrics_router = Router::new()
            .route("/metrics", get(crate::metrics::metrics_handler))
            .with_state(metrics_state);

        // Combine all routes
        let app = Router::new()
            .merge(api_routes)
            .merge(health_router)
            .merge(metrics_router);

        // Add middleware layers
        app.layer(TraceLayer::new_for_http())
            .layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            )
    }

    /// Get the health checker
    pub fn health_checker(&self) -> Arc<HealthChecker> {
        self.health_checker.clone()
    }

    /// Get the metrics collector
    pub fn metrics_collector(&self) -> Arc<MetricsCollector> {
        self.metrics_collector.clone()
    }

    /// Run the server
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()?;

        let listener = TcpListener::bind(addr).await?;
        info!("Server listening on {}", addr);

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        // Setup signal handlers
        let shutdown_tx_clone = shutdown_tx.clone();
        tokio::spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    info!("Received shutdown signal");
                    let _ = shutdown_tx_clone.send(());
                }
                Err(e) => {
                    error!("Failed to listen for shutdown signal: {}", e);
                }
            }
        });

        // Also handle SIGTERM on Unix
        #[cfg(unix)]
        tokio::spawn(async move {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate())?;
            sigterm.recv().await;
            info!("Received SIGTERM");
            let _ = shutdown_tx.send(());
            Ok::<(), std::io::Error>(())
        });

        // Clone for the server task
        let app = self.app;

        // Run the server
        let server_future = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                shutdown_rx.recv().await.ok();
                info!("Shutting down server...");
            });

        server_future.await?;

        info!("Server stopped gracefully");
        Ok(())
    }

    /// Run the server with a custom shutdown future
    pub async fn run_with_shutdown<F>(self, shutdown_future: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()?;

        let listener = TcpListener::bind(addr).await?;
        info!("Server listening on {}", addr);

        let server_future = axum::serve(listener, self.app)
            .with_graceful_shutdown(async move {
                shutdown_future.await;
                info!("Shutting down server...");
            });

        server_future.await?;

        info!("Server stopped gracefully");
        Ok(())
    }
}

/// Root handler
async fn root_handler() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "service": "nebula-production",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running"
    }))
}

/// Ping handler for simple health checks
async fn ping_handler() -> &'static str {
    "pong"
}

/// Start the server from configuration
pub async fn start_server(config: &ProductionConfig) -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::new(config)?;
    server.run().await
}

