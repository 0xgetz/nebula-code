//! Nebula Production - Production-ready deployment infrastructure for Rust applications.
//!
//! This crate provides essential components for deploying production-grade services with
//! security, observability, and performance built in.
//!
//! # Overview
//!
//! Nebula Production is a comprehensive toolkit for building production-ready Rust services.
//! It covers the full spectrum of production concerns:
//!
//! - **Configuration Management** - Environment-aware configuration with validation
//! - **Health Checking** - Liveness and readiness probes for Kubernetes
//! - **Security** - TLS, API keys, JWT authentication, input validation, CSRF protection
//! - **Authentication & Authorization** - RBAC, password hashing, JWT token management
//! - **Observability** - Structured logging, metrics collection, distributed tracing
//! - **Performance** - Connection pooling, caching, load balancing, rate limiting
//! - **Alerting** - Severity-based alerting with notification channels
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use nebula_production::{
//!     ProductionConfig, HealthChecker, Server, SecurityConfig,
//!     init_logging, MetricsCollector,
//! };
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load configuration from environment
//!     let config = ProductionConfig::from_env()?;
//!
//!     // Initialize logging
//!     init_logging(&config.logging.level, &config.logging.format);
//!
//!     // Create health checker
//!     let health_checker = Arc::new(HealthChecker::new());
//!
//!     // Create metrics collector
//!     let metrics = MetricsCollector::new();
//!
//!     // Configure security
//!     let security = SecurityConfig::production();
//!
//!     // Build and start the server
//!     let server = Server::builder()
//!         .config(config.clone())
//!         .health_checker(health_checker)
//!         .metrics(metrics)
//!         .security(security)
//!         .build()?;
//!
//!     server.start().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Module Overview
//!
//! - [`config`] - Environment-aware configuration with validation
//! - [`health`] - Health checking with readiness and liveness probes
//! - [`metrics`] - Prometheus-compatible metrics collection
//! - [`logging`] - Structured JSON logging with tracing integration
//! - [`security`] - TLS, API keys, JWT, input validation, CSRF protection
//! - [`auth`] - Authentication middleware and RBAC authorization
//! - [`pool`] - Generic connection pooling for databases and HTTP clients
//! - [`cache`] - In-memory and Redis-backed caching with eviction policies
//! - [`rate_limit`] - Token bucket rate limiting with tiered limits
//! - [`tracing`] - OpenTelemetry-compatible distributed tracing
//! - [`alerting`] - Alert management with severity levels and notifications
//! - [`observability`] - Unified observability stack configuration
//! - [`optimization`] - Performance optimization configuration
//! - [`load_balancer`] - Load balancing strategies for distributed systems
//! - [`encryption`] - Encryption utilities with HMAC and key derivation
//! - [`server`] - HTTP server with graceful shutdown
//!
//! # Configuration
//!
//! Configuration is loaded from environment variables with sensible defaults:
//!
//! | Variable | Description | Default |
//! |----------|-------------|---------|
//! | `NEBULA_ENV` | Environment (development/staging/production) | development |
//! | `NEBULA_HOST` | Host to bind to | 0.0.0.0 |
//! | `NEBULA_PORT` | Port to listen on | 8080 |
//! | `RUST_LOG` | Log level (trace/debug/info/warn/error) | info |
//! | `NEBULA_LOG_FORMAT` | Log format (json/pretty) | json |
//! | `NEBULA_APP_NAME` | Application name | nebula-production |
//! | `NEBULA_APP_VERSION` | Application version | 0.1.0 |
//! | `NEBULA_SHUTDOWN_TIMEOUT` | Graceful shutdown timeout in seconds | 30 |
//!
//! # Security
//!
//! The [`SecurityConfig`] provides production-safe defaults:
//!
//! ```rust
//! use nebula_production::SecurityConfig;
//!
//! // Use production-safe defaults
//! let config = SecurityConfig::production();
//! assert!(config.tls.enabled);
//! assert_eq!(config.tls.min_version, nebula_production::TlsVersion::Tls13);
//! ```
//!
//! # License
//!
//! MIT

// Ensure documentation covers all public items
#![warn(missing_docs)]

pub mod alerting;
pub mod auth;
pub mod cache;
pub mod config;
pub mod encryption;
pub mod health;
pub mod load_balancer;
pub mod logging;
pub mod metrics;
pub mod observability;
pub mod optimization;
pub mod pool;
pub mod rate_limit;
pub mod security;
pub mod server;
pub mod tracing;

// Re-export commonly used types for convenience
pub use alerting::{Alert, AlertManager, AlertOperator, AlertRule, AlertStatus, NotificationChannel, Severity};
pub use auth::{
    AuthError, AuthMethod, AuthMiddleware, AuthenticationResult, Authorization, InMemoryAuthorization,
    RbacConfig, RoleDefinition, hash_password, verify_password,
};
pub use cache::{
    CacheBackend, CacheConfig, CacheEntry, CacheError, CacheResult, CacheStats, EvictionPolicy,
    InMemoryCache, RedisConfig, ThreadSafeCache,
};
pub use config::{Environment, ProductionConfig, ServerConfigInner};
pub use encryption::{
    EncryptionConfig, EncryptionEngine, EncryptedData, TlsConfigBuilder,
    compute_hmac, derive_key_from_password, generate_nonce, generate_random_key, verify_hmac,
};
pub use health::{HealthCheck, HealthChecker, HealthStatus, ServiceInfo};
pub use load_balancer::{
    LoadBalancer, LoadBalancerError, LoadBalancerResult, LoadBalancerStats, ServerNode,
};
pub use logging::init_logging;
pub use metrics::MetricsCollector;
pub use observability::{AlertingConfig, LogConfig, LogFormat, ObservabilityBuilder, ObservabilityConfig, ObservabilityStack};
pub use optimization::{
    LoadBalancingStrategy, OptimizationLevel, PerformanceConfig, PerformanceHealth,
    PerformanceMetrics,
};
pub use pool::{
    ConnectionFactory, ConnectionPool, PoolConfig, PoolError, PoolResult, PoolStats,
    PooledConnection, PooledConnectionGuard,
};
pub use rate_limit::{
    RateLimitError, RateLimitKeyStrategy, RateLimitResult, RateLimiter, RateLimiterConfig,
    RateLimitMiddleware, RateLimitTier,
};
pub use security::{
    ApiKeyConfig, ApiKeyEntry, CorsConfig, JwtAlgorithm, JwtConfig, SecurityConfig,
    SecurityError, SecurityHeaders, TlsConfig, TlsVersion,
};
pub use server::{Server, ServerConfig};
pub use tracing::{
    init_tracing, log_trace_event, trace_operation, ExporterConfig, SpanAttributes,
    TraceContext, TraceGuard, TracingConfig,
};
