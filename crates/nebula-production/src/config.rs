//! Configuration management with environment-based settings.
//!
//! This module provides centralized configuration management supporting:
//!
//! # Features
//!
//! - **Environment-Aware** - Different settings for development, staging, and production
//! - **Environment Variable Support** - Configuration via standard env vars
//! - **Validation** - Automatic validation of configuration values
//! - **Global Configuration** - Thread-safe global configuration access
//! - **Serde Integration** - Easy serialization/deserialization for config files
//!
//! # Environment Variables
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
//! # Quick Start
//!
//! ```rust
//! use nebula_production::{ProductionConfig, Environment};
//!
//! // Load from environment variables
//! let config = ProductionConfig::from_env().expect("Failed to load config");
//!
//! // Or create programmatically
//! let config = ProductionConfig {
//!     environment: Environment::Production,
//!     app_name: "my-service".to_string(),
//!     app_version: "1.0.0".to_string(),
//!     ..Default::default()
//! };
//!
//! // Validate before use
//! config.validate().expect("Invalid configuration");
//! ```
//!
//! # Global Configuration
//!
//! For applications that need global configuration access:
//!
//! ```rust,ignore
//! use nebula_production::ProductionConfig;
//!
//! // Initialize once at startup
//! ProductionConfig::init().expect("Failed to init config");
//!
//! // Access globally
//! let config = ProductionConfig::get().expect("Config not initialized");
//! println!("Running in {} mode", config.environment);
//! ```
//!
//! # Environment Detection
//!
//! ```rust
//! use nebula_production::Environment;
//!
//! let env = Environment::from_str("production");
//! assert!(env.is_production());
//! assert!(!env.is_development());
//!
//! let env = Environment::from_str("dev");  // Also accepts aliases
//! assert!(env.is_development());
//! ```
//!
//! # Configuration File Support
//!
//! Configuration can be loaded from TOML, JSON, or YAML files:
//!
//! ```toml
//! # config.toml
//! environment = "production"
//! app_name = "my-service"
//! app_version = "1.0.0"
//!
//! [server]
//! host = "0.0.0.0"
//! port = 8080
//! shutdown_timeout_secs = 30
//!
//! [logging]
//! level = "info"
//! format = "json"
//! ```
//!
//! ```rust,ignore
//! use nebula_production::ProductionConfig;
//!
//! let config_str = std::fs::read_to_string("config.toml")?;
//! let config: ProductionConfig = toml::from_str(&config_str)?;
//! ```


use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use thiserror::Error;

/// Supported deployment environments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    #[default]
    Development,
    Staging,
    Production,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Staging => write!(f, "staging"),
            Environment::Production => write!(f, "production"),
        }
    }
}

impl Environment {
    /// Parse environment from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Environment::Development,
            "staging" | "stage" => Environment::Staging,
            "production" | "prod" => Environment::Production,
            _ => Environment::Development,
        }
    }

    /// Check if this is a production environment
    pub fn is_production(&self) -> bool {
        matches!(self, Environment::Production)
    }

    /// Check if this is a development environment
    pub fn is_development(&self) -> bool {
        matches!(self, Environment::Development)
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfigInner {
    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,
    /// Port to listen on
    #[serde(default = "default_port")]
    pub port: u16,
    /// Graceful shutdown timeout in seconds
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_shutdown_timeout() -> u64 {
    30
}

impl Default for ServerConfigInner {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            shutdown_timeout_secs: default_shutdown_timeout(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,
    /// Log format (json, pretty)
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    #[serde(default = "default_metrics_enabled")]
    pub enabled: bool,
    /// Metrics endpoint path
    #[serde(default = "default_metrics_path")]
    pub path: String,
    /// Metrics collection interval in seconds
    #[serde(default = "default_metrics_interval")]
    pub interval_secs: u64,
}

fn default_metrics_enabled() -> bool {
    true
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

fn default_metrics_interval() -> u64 {
    60
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_metrics_enabled(),
            path: default_metrics_path(),
            interval_secs: default_metrics_interval(),
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthConfig {
    /// Enable health endpoints
    #[serde(default = "default_health_enabled")]
    pub enabled: bool,
    /// Liveness probe endpoint
    #[serde(default = "default_liveness_path")]
    pub liveness_path: String,
    /// Readiness probe endpoint
    #[serde(default = "default_readiness_path")]
    pub readiness_path: String,
    /// Health check interval in seconds
    #[serde(default = "default_health_interval")]
    pub interval_secs: u64,
}

fn default_health_enabled() -> bool {
    true
}

fn default_liveness_path() -> String {
    "/health/live".to_string()
}

fn default_readiness_path() -> String {
    "/health/ready".to_string()
}

fn default_health_interval() -> u64 {
    10
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            enabled: default_health_enabled(),
            liveness_path: default_liveness_path(),
            readiness_path: default_readiness_path(),
            interval_secs: default_health_interval(),
        }
    }
}

/// Main production configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductionConfig {
    /// Deployment environment
    #[serde(default)]
    pub environment: Environment,
    /// Application name
    #[serde(default = "default_app_name")]
    pub app_name: String,
    /// Application version
    #[serde(default = "default_app_version")]
    pub app_version: String,
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfigInner,
    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
    /// Metrics configuration
    #[serde(default)]
    pub metrics: MetricsConfig,
    /// Health check configuration
    #[serde(default)]
    pub health: HealthConfig,
}

fn default_app_name() -> String {
    "nebula-production".to_string()
}

fn default_app_version() -> String {
    "0.1.0".to_string()
}

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to load configuration: {0}")]
    LoadError(String),
    #[error("Invalid configuration: {0}")]
    ValidationError(String),
}

static CONFIG: OnceLock<ProductionConfig> = OnceLock::new();

impl ProductionConfig {
    /// Load configuration from environment variables and defaults
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment = std::env::var("NEBULA_ENV")
            .map(|s| Environment::from_str(&s))
            .unwrap_or_default();

        let host = std::env::var("NEBULA_HOST").unwrap_or_else(|_| default_host());
        let port = std::env::var("NEBULA_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or_else(default_port);

        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| default_log_level());
        let log_format = std::env::var("NEBULA_LOG_FORMAT").unwrap_or_else(|_| default_log_format());

        let config = Self {
            environment,
            app_name: std::env::var("NEBULA_APP_NAME").unwrap_or_else(|_| default_app_name()),
            app_version: std::env::var("NEBULA_APP_VERSION").unwrap_or_else(|_| default_app_version()),
            server: ServerConfigInner {
                host,
                port,
                shutdown_timeout_secs: std::env::var("NEBULA_SHUTDOWN_TIMEOUT")
                    .ok()
                    .and_then(|t| t.parse().ok())
                    .unwrap_or_else(default_shutdown_timeout),
            },
            logging: LoggingConfig {
                level: log_level,
                format: log_format,
            },
            metrics: MetricsConfig::default(),
            health: HealthConfig::default(),
        };

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.server.port == 0 {
            return Err(ConfigError::ValidationError(
                "Port cannot be zero".to_string(),
            ));
        }

        if self.server.shutdown_timeout_secs == 0 {
            return Err(ConfigError::ValidationError(
                "Shutdown timeout must be positive".to_string(),
            ));
        }

        Ok(())
    }

    /// Get or initialize the global configuration
    pub fn get() -> Result<&'static ProductionConfig, ConfigError> {
        if let Some(config) = CONFIG.get() {
            Ok(config)
        } else {
            let config = Self::from_env()?;
            CONFIG.set(config).map_err(|_| {
                ConfigError::LoadError("Configuration already initialized".to_string())
            })?;
            Ok(CONFIG.get().unwrap())
        }
    }

    /// Initialize the global configuration
    pub fn init() -> Result<(), ConfigError> {
        let config = Self::from_env()?;
        CONFIG.set(config).map_err(|_| {
            ConfigError::LoadError("Configuration already initialized".to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_parsing() {
        assert_eq!(Environment::from_str("development"), Environment::Development);
        assert_eq!(Environment::from_str("dev"), Environment::Development);
        assert_eq!(Environment::from_str("staging"), Environment::Staging);
        assert_eq!(Environment::from_str("stage"), Environment::Staging);
        assert_eq!(Environment::from_str("production"), Environment::Production);
        assert_eq!(Environment::from_str("prod"), Environment::Production);
        assert_eq!(Environment::from_str("unknown"), Environment::Development);
    }

    #[test]
    fn test_environment_predicates() {
        assert!(Environment::Development.is_development());
        assert!(!Environment::Development.is_production());
        assert!(Environment::Production.is_production());
        assert!(!Environment::Production.is_development());
    }

    #[test]
    fn test_default_config() {
        let config = ProductionConfig::default();
        assert_eq!(config.environment, Environment::Development);
        assert_eq!(config.server.port, 8080);
        assert!(config.metrics.enabled);
        assert!(config.health.enabled);
    }
}
