//! Structured logging with the tracing crate
//!
//! Provides:
//! - JSON and pretty formatting
//! - Environment-based log levels
//! - Structured fields for production monitoring

use crate::config::{LoggingConfig, ProductionConfig};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer, Registry};

/// Initialize logging based on configuration
pub fn init_logging(config: &LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(&config.level)
    });

    let fmt_layer = if config.format == "json" {
        fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .boxed()
    } else {
        fmt::layer()
            .pretty()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .boxed()
    };

    Registry::default()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    Ok(())
}

/// Initialize logging from environment variables
pub fn init_logging_from_env() -> Result<(), Box<dyn std::error::Error>> {
    let config = ProductionConfig::from_env()?;
    init_logging(&config.logging)
}

/// Log a structured event with custom fields
#[macro_export]
macro_rules! log_event {
    (level: $level:expr, message: $message:expr, $($key:ident = $value:expr),* $(,)?) => {{
        tracing::event!(
            target: module_path!(),
            $level,
            $($key = %$value),*,
            "{}",
            $message
        );
    }};
}

/// Log an info event
#[macro_export]
macro_rules! info_event {
    ($message:expr $(, $key:ident = $value:expr)*) => {{
        log_event!(level: tracing::Level::INFO, message: $message $(, $key = $value)*);
    }};
}

/// Log an error event
#[macro_export]
macro_rules! error_event {
    ($message:expr $(, $key:ident = $value:expr)*) => {{
        log_event!(level: tracing::Level::ERROR, message: $message $(, $key = $value)*);
    }};
}

/// Log a warning event
#[macro_export]
macro_rules! warn_event {
    ($message:expr $(, $key:ident = $value:expr)*) => {{
        log_event!(level: tracing::Level::WARN, message: $message $(, $key = $value)*);
    }};
}

/// Log a debug event
#[macro_export]
macro_rules! debug_event {
    ($message:expr $(, $key:ident = $value:expr)*) => {{
        log_event!(level: tracing::Level::DEBUG, message: $message $(, $key = $value)*);
    }};
}

/// Span for tracking request lifecycle
pub struct RequestSpan {
    span: tracing::Span,
}

impl RequestSpan {
    /// Create a new request span
    pub fn new(
        method: &str,
        path: &str,
        request_id: &str,
    ) -> Self {
        let span = tracing::info_span!(
            "http_request",
            http.method = method,
            http.url = path,
            http.request_id = request_id,
            otel.kind = "server",
        );
        Self { span }
    }

    /// Enter the span
    pub fn enter(&self) -> tracing::span::Entered<'_> {
        self.span.enter()
    }

    /// Record additional fields
    pub fn record<K, V>(&self, key: K, value: V) -> &Self
    where
        K: AsRef<str>,
        V: tracing::Value,
    {
        self.span.record(key.as_ref(), value);
        self
    }

    /// Record the response status
    pub fn record_status(&self, status: u16) -> &Self {
        self.record("http.status_code", status)
    }

    /// Record an error
    pub fn record_error(&self, error: &dyn std::error::Error) -> &Self {
        self.record("error.message", error.to_string())
            .record("error.kind", "internal")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_creation() {
        let config = LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
        };
        assert_eq!(config.level, "info");
        assert_eq!(config.format, "json");
    }

    #[test]
    fn test_logging_config_pretty() {
        let config = LoggingConfig {
            level: "debug".to_string(),
            format: "pretty".to_string(),
        };
        assert_eq!(config.level, "debug");
        assert_eq!(config.format, "pretty");
    }
}
