//! Unified observability configuration
//!
//! Combines metrics, logging, and tracing into a single observability stack
//! for production deployments.

use crate::alerting::{AlertManager, AlertRule, Severity};
use crate::config::{Environment, LoggingConfig};
use crate::logging::init_logging;
use crate::metrics::MetricsCollector;
use crate::tracing::{init_tracing, ExporterConfig, TracingConfig};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info};

/// Errors that can occur in observability
#[derive(Debug, Error)]
pub enum ObservabilityError {
    #[error("Failed to initialize observability: {0}")]
    InitializationFailed(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Alert error: {0}")]
    AlertError(crate::alerting::AlertError),
}

impl From<crate::alerting::AlertError> for ObservabilityError {
    fn from(err: crate::alerting::AlertError) -> Self {
        ObservabilityError::AlertError(err)
    }
}

/// Logging format for observability config
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(PartialEq, Eq)]
pub enum LogFormat {
    /// Pretty human-readable format
    #[default]
    Pretty,
    /// JSON format for machine parsing
    Json,
}

/// Logging configuration for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log format (pretty or json)
    #[serde(default)]
    pub format: LogFormat,
    /// Log level
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Pretty,
            level: "info".to_string(),
        }
    }
}

impl LogConfig {
    /// Convert to LoggingConfig for initialization
    pub fn to_logging_config(&self) -> LoggingConfig {
        LoggingConfig {
            level: self.level.clone(),
            format: match self.format {
                LogFormat::Pretty => "pretty".to_string(),
                LogFormat::Json => "json".to_string(),
            },
        }
    }
}

/// Unified observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Service name (used across all observability components)
    pub service_name: String,
    /// Environment (development, staging, production)
    pub environment: Environment,
    /// Logging configuration
    pub logging: LogConfig,
    /// Metrics configuration
    pub metrics: MetricsConfigInner,
    /// Tracing configuration
    pub tracing: TracingConfig,
    /// Alerting configuration
    pub alerting: AlertingConfig,
}

/// Simplified metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfigInner {
    /// Whether metrics are enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Metrics endpoint path
    #[serde(default = "default_metrics_endpoint")]
    pub endpoint: String,
}

fn default_true() -> bool {
    true
}

fn default_metrics_endpoint() -> String {
    "/metrics".to_string()
}

impl Default for MetricsConfigInner {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: "/metrics".to_string(),
        }
    }
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            service_name: "nebula-service".to_string(),
            environment: Environment::Development,
            logging: LogConfig::default(),
            metrics: MetricsConfigInner::default(),
            tracing: TracingConfig::default(),
            alerting: AlertingConfig::default(),
        }
    }
}

impl ObservabilityConfig {
    /// Create a new observability config for the given environment
    pub fn for_environment(env: Environment, service_name: impl Into<String>) -> Self {
        let service_name = service_name.into();
        match env {
            Environment::Development => Self {
                service_name: service_name.clone(),
                environment: env,
                logging: LogConfig {
                    format: LogFormat::Pretty,
                    level: "debug".to_string(),
                    ..Default::default()
                },
                metrics: MetricsConfigInner {
                    enabled: true,
                    endpoint: "/metrics".to_string(),
                },
                tracing: TracingConfig {
                    service_name,
                    sample_rate: 1.0,
                    exporter: ExporterConfig::Console {
                        pretty: true,
                        with_timestamps: true,
                    },
                    ..Default::default()
                },
                alerting: AlertingConfig {
                    enabled: false,
                    ..Default::default()
                },
            },
            Environment::Staging => Self {
                service_name: service_name.clone(),
                environment: env,
                logging: LogConfig {
                    format: LogFormat::Json,
                    level: "info".to_string(),
                    ..Default::default()
                },
                metrics: MetricsConfigInner {
                    enabled: true,
                    endpoint: "/metrics".to_string(),
                },
                tracing: TracingConfig {
                    service_name,
                    sample_rate: 0.5,
                    exporter: ExporterConfig::Console {
                        pretty: false,
                        with_timestamps: true,
                    },
                    ..Default::default()
                },
                alerting: AlertingConfig {
                    enabled: true,
                    ..Default::default()
                },
            },
            Environment::Production => Self {
                service_name: service_name.clone(),
                environment: env,
                logging: LogConfig {
                    format: LogFormat::Json,
                    level: "warn".to_string(),
                    ..Default::default()
                },
                metrics: MetricsConfigInner {
                    enabled: true,
                    endpoint: "/metrics".to_string(),
                },
                tracing: TracingConfig {
                    service_name,
                    sample_rate: 0.1,
                    exporter: ExporterConfig::Console {
                        pretty: false,
                        with_timestamps: true,
                    },
                    ..Default::default()
                },
                alerting: AlertingConfig {
                    enabled: true,
                    ..Default::default()
                },
            },
        }
    }

    /// Build the observability stack
    pub async fn build(self) -> Result<ObservabilityStack, ObservabilityError> {
        info!(service = %self.service_name, env = %self.environment, "Building observability stack");

        // Initialize logging
        let logging_config = self.logging.to_logging_config();
        init_logging(&logging_config).map_err(|e| ObservabilityError::InitializationFailed(e.to_string()))?;

        // Initialize metrics collector
        let metrics_collector = if self.metrics.enabled {
            Some(MetricsCollector::new())
        } else {
            None
        };

        // Initialize tracing
        init_tracing(&self.tracing).map_err(|e| ObservabilityError::InitializationFailed(e.to_string()))?;

        // Initialize alerting if enabled
        let alert_manager = if self.alerting.enabled {
            let manager = AlertManager::new();

            // Add default console channel
            manager
                .add_channel(
                    "console".to_string(),
                    crate::alerting::NotificationChannel::Console { include_timestamp: true },
                )
                .await;

            // Add default alert rules
            self.add_default_alert_rules(&manager).await;

            Some(Arc::new(RwLock::new(manager)))
        } else {
            None
        };

        info!("Observability stack initialized successfully");

        Ok(ObservabilityStack {
            config: self,
            alert_manager,
            metrics_collector,
        })
    }

    /// Add default alert rules to the alert manager
    async fn add_default_alert_rules(&self, manager: &AlertManager) {
        // High CPU usage
        manager
            .add_rule(AlertRule {
                id: "high-cpu".to_string(),
                name: "High CPU Usage".to_string(),
                metric: "cpu_usage_percent".to_string(),
                operator: crate::alerting::AlertOperator::Gt,
                threshold: 80.0,
                severity: Severity::High,
                for_duration_secs: 60,
                channels: vec!["console".to_string()],
                labels: [("team".to_string(), "platform".to_string())].into_iter().collect(),
            })
            .await;

        // High memory usage
        manager
            .add_rule(AlertRule {
                id: "high-memory".to_string(),
                name: "High Memory Usage".to_string(),
                metric: "memory_usage_percent".to_string(),
                operator: crate::alerting::AlertOperator::Gt,
                threshold: 90.0,
                severity: Severity::Critical,
                for_duration_secs: 30,
                channels: vec!["console".to_string()],
                labels: [("team".to_string(), "platform".to_string())].into_iter().collect(),
            })
            .await;

        // High error rate
        manager
            .add_rule(AlertRule {
                id: "high-error-rate".to_string(),
                name: "High Error Rate".to_string(),
                metric: "error_rate".to_string(),
                operator: crate::alerting::AlertOperator::Gt,
                threshold: 5.0,
                severity: Severity::High,
                for_duration_secs: 120,
                channels: vec!["console".to_string()],
                labels: [("team".to_string(), "application".to_string())].into_iter().collect(),
            })
            .await;
    }
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Whether alerting is enabled
    #[serde(default)]
    pub enabled: bool,
    /// Default notification channels
    #[serde(default)]
    pub default_channels: Vec<String>,
    /// Alert evaluation interval (seconds)
    #[serde(default = "default_evaluation_interval")]
    pub evaluation_interval_secs: u64,
}

fn default_evaluation_interval() -> u64 {
    60
}

impl Default for AlertingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_channels: Vec::new(),
            evaluation_interval_secs: 60,
        }
    }
}

/// The complete observability stack
pub struct ObservabilityStack {
    config: ObservabilityConfig,
    alert_manager: Option<Arc<RwLock<AlertManager>>>,
    metrics_collector: Option<MetricsCollector>,
}

impl ObservabilityStack {
    /// Get the configuration
    pub fn config(&self) -> &ObservabilityConfig {
        &self.config
    }

    /// Get the alert manager (if alerting is enabled)
    pub fn alert_manager(&self) -> Option<&Arc<RwLock<AlertManager>>> {
        self.alert_manager.as_ref()
    }

    /// Get a cloned reference to the alert manager
    pub fn alert_manager_cloned(&self) -> Option<Arc<RwLock<AlertManager>>> {
        self.alert_manager.clone()
    }

    /// Get the metrics collector
    pub fn metrics_collector(&self) -> Option<&MetricsCollector> {
        self.metrics_collector.as_ref()
    }

    /// Get a mutable reference to the metrics collector
    pub fn metrics_collector_mut(&mut self) -> Option<&mut MetricsCollector> {
        self.metrics_collector.as_mut()
    }

    /// Shutdown the observability stack gracefully
    pub async fn shutdown(&self) {
        info!("Shutting down observability stack");
        // In a full implementation, this would:
        // - Flush any pending log entries
        // - Export remaining spans
        // - Send final metrics
        // - Gracefully shutdown alert managers
    }
}

impl Drop for ObservabilityStack {
    fn drop(&mut self) {
        // Best-effort cleanup on drop
        info!("Observability stack dropped");
    }
}

/// Builder for observability configuration
pub struct ObservabilityBuilder {
    service_name: String,
    environment: Environment,
    log_level: Option<String>,
    metrics_enabled: bool,
    tracing_sample_rate: f64,
    alerting_enabled: bool,
}

impl Default for ObservabilityBuilder {
    fn default() -> Self {
        Self {
            service_name: "nebula-service".to_string(),
            environment: Environment::Development,
            log_level: None,
            metrics_enabled: true,
            tracing_sample_rate: 1.0,
            alerting_enabled: false,
        }
    }
}

impl ObservabilityBuilder {
    /// Create a new builder
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            ..Default::default()
        }
    }

    /// Set the environment
    pub fn environment(mut self, env: Environment) -> Self {
        self.environment = env;
        self
    }

    /// Set the log level
    pub fn log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = Some(level.into());
        self
    }

    /// Enable or disable metrics
    pub fn with_metrics(mut self, enabled: bool) -> Self {
        self.metrics_enabled = enabled;
        self
    }

    /// Set the tracing sample rate
    pub fn sample_rate(mut self, rate: f64) -> Self {
        self.tracing_sample_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable alerting
    pub fn with_alerting(mut self, enabled: bool) -> Self {
        self.alerting_enabled = enabled;
        self
    }

    /// Build the observability configuration
    pub fn build_config(self) -> ObservabilityConfig {
        let mut config = ObservabilityConfig::for_environment(self.environment, self.service_name);

        if let Some(level) = self.log_level {
            config.logging.level = level;
        }

        config.metrics.enabled = self.metrics_enabled;
        config.tracing.sample_rate = self.tracing_sample_rate;
        config.alerting.enabled = self.alerting_enabled;

        config
    }

    /// Build and initialize the observability stack
    pub async fn build(self) -> Result<ObservabilityStack, ObservabilityError> {
        self.build_config().build().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observability_builder_config() {
        let config = ObservabilityBuilder::new("test-service")
            .environment(Environment::Staging)
            .log_level("debug")
            .with_metrics(true)
            .sample_rate(0.5)
            .with_alerting(true)
            .build_config();

        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.environment, Environment::Staging);
        assert_eq!(config.logging.level, "debug");
        assert!(config.metrics.enabled);
        assert_eq!(config.tracing.sample_rate, 0.5);
        assert!(config.alerting.enabled);
    }

    #[test]
    fn test_observability_config_for_environment() {
        let dev = ObservabilityConfig::for_environment(Environment::Development, "test");
        assert_eq!(dev.logging.format, LogFormat::Pretty);
        assert!(!dev.alerting.enabled);

        let prod = ObservabilityConfig::for_environment(Environment::Production, "test");
        assert_eq!(prod.logging.format, LogFormat::Json);
        assert!(prod.alerting.enabled);
    }

    #[test]
    fn test_alerting_config_default() {
        let config = AlertingConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.evaluation_interval_secs, 60);
        assert!(config.default_channels.is_empty());
    }

    #[test]
    fn test_metrics_config_inner_default() {
        let config = MetricsConfigInner::default();
        assert!(config.enabled);
        assert_eq!(config.endpoint, "/metrics");
    }
}
