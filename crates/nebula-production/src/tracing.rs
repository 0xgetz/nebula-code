//! Distributed tracing with OpenTelemetry integration
//!
//! Provides tracing infrastructure for observability across services,
//! including span management, context propagation, and exporter configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use thiserror::Error;
use tracing::{debug, error, info, instrument, warn, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Registry};

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Service name for tracing
    pub service_name: String,
    /// Sampling rate (0.0 to 1.0)
    #[serde(default = "default_sample_rate")]
    pub sample_rate: f64,
    /// Exporter configuration
    pub exporter: ExporterConfig,
    /// Whether to include local spans
    #[serde(default = "default_true")]
    pub include_local_spans: bool,
    /// Maximum number of spans to buffer
    #[serde(default = "default_max_spans")]
    pub max_buffered_spans: usize,
}

fn default_sample_rate() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_max_spans() -> usize {
    10000
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "nebula-service".to_string(),
            sample_rate: 1.0,
            exporter: ExporterConfig::default(),
            include_local_spans: true,
            max_buffered_spans: 10000,
        }
    }
}

/// Exporter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ExporterConfig {
    /// No exporter (tracing only)
    None,
    /// Output to stdout
    Console {
        /// Pretty print spans
        #[serde(default = "default_true")]
        pretty: bool,
        /// Include timestamps
        #[serde(default = "default_true")]
        with_timestamps: bool,
    },
    /// Jaeger exporter
    Jaeger {
        /// Jaeger agent endpoint
        endpoint: String,
    },
    /// OpenTelemetry HTTP exporter (e.g., for Tempo, Lightstep)
    Http {
        /// Endpoint URL
        endpoint: String,
        /// Headers to include
        headers: Option<HashMap<String, String>>,
    },
}

impl Default for ExporterConfig {
    fn default() -> Self {
        ExporterConfig::Console {
            pretty: true,
            with_timestamps: true,
        }
    }
}

/// Errors that can occur in tracing
#[derive(Debug, Error)]
pub enum TracingError {
    #[error("Failed to initialize tracing: {0}")]
    InitializationFailed(String),
    #[error("Failed to export spans: {0}")]
    ExportFailed(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Trace context for propagating trace information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraceContext {
    /// Trace ID
    pub trace_id: String,
    /// Span ID
    pub span_id: String,
    /// Parent span ID (if any)
    pub parent_span_id: Option<String>,
    /// Whether this trace is sampled
    pub sampled: bool,
    /// Additional baggage items
    pub baggage: HashMap<String, String>,
}

impl TraceContext {
    /// Create a new trace context
    pub fn new(trace_id: String, span_id: String) -> Self {
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            sampled: true,
            baggage: HashMap::new(),
        }
    }

    /// Create from existing context (for distributed tracing)
    pub fn from_headers(headers: &HashMap<String, String>) -> Self {
        Self {
            trace_id: headers.get("traceparent").cloned().unwrap_or_default(),
            span_id: headers.get("span_id").cloned().unwrap_or_default(),
            parent_span_id: headers.get("parent_span_id").cloned(),
            sampled: headers.get("sampled").map(|s| s == "true").unwrap_or(true),
            baggage: headers
                .iter()
                .filter(|(k, _)| k.starts_with("baggage-"))
                .map(|(k, v)| (k.trim_start_matches("baggage-").to_string(), v.clone()))
                .collect(),
        }
    }

    /// Convert to headers for propagation
    pub fn to_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("traceparent".to_string(), self.trace_id.clone());
        headers.insert("span_id".to_string(), self.span_id.clone());
        if let Some(parent) = &self.parent_span_id {
            headers.insert("parent_span_id".to_string(), parent.clone());
        }
        headers.insert("sampled".to_string(), self.sampled.to_string());
        for (k, v) in &self.baggage {
            headers.insert(format!("baggage-{}", k), v.clone());
        }
        headers
    }
}

/// Span attributes builder
#[derive(Debug, Clone, Default)]
pub struct SpanAttributes {
    attributes: HashMap<String, String>,
}

impl SpanAttributes {
    /// Create new attributes
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an attribute
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Add HTTP method attribute
    pub fn with_http_method(mut self, method: &str) -> Self {
        self.attributes.insert("http.method".to_string(), method.to_string());
        self
    }

    /// Add HTTP URL attribute
    pub fn with_http_url(mut self, url: &str) -> Self {
        self.attributes.insert("http.url".to_string(), url.to_string());
        self
    }

    /// Add HTTP status code attribute
    pub fn with_http_status(mut self, status: u16) -> Self {
        self.attributes.insert("http.status_code".to_string(), status.to_string());
        self
    }

    /// Add database statement attribute
    pub fn with_db_statement(mut self, statement: &str) -> Self {
        self.attributes.insert("db.statement".to_string(), statement.to_string());
        self
    }

    /// Add peer service attribute
    pub fn with_peer_service(mut self, service: &str) -> Self {
        self.attributes.insert("peer.service".to_string(), service.to_string());
        self
    }

    /// Get the attributes
    pub fn as_map(&self) -> &HashMap<String, String> {
        &self.attributes
    }
}

/// Initialize distributed tracing
#[instrument(skip(config), fields(service = config.service_name))]
pub fn init_tracing(config: &TracingConfig) -> Result<(), TracingError> {
    info!(service = %config.service_name, "Initializing distributed tracing");

    // Create the filter layer
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Build the tracing subscriber based on exporter config
    match &config.exporter {
        ExporterConfig::None => {
            // No exporter, just local tracing
            let subscriber = Registry::default().with(filter);
            subscriber.try_init().map_err(|e| TracingError::InitializationFailed(e.to_string()))?;
        }
        ExporterConfig::Console { pretty, with_timestamps: _ } => {
            let fmt_layer = fmt::layer()
                .with_ansi(true)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true);

            if *pretty {
                let subscriber = Registry::default()
                    .with(filter)
                    .with(fmt_layer.pretty());
                subscriber.try_init().map_err(|e| TracingError::InitializationFailed(e.to_string()))?;
            } else {
                let subscriber = Registry::default()
                    .with(filter)
                    .with(fmt_layer.compact());
                subscriber.try_init().map_err(|e| TracingError::InitializationFailed(e.to_string()))?;
            }
        }
        ExporterConfig::Jaeger { endpoint } => {
            // For Jaeger, we'd use opentelemetry-jaeger crate
            // This is a placeholder for the integration
            debug!(endpoint = %endpoint, "Jaeger exporter configured (placeholder)");

            let subscriber = Registry::default().with(filter);
            subscriber.try_init().map_err(|e| TracingError::InitializationFailed(e.to_string()))?;
        }
        ExporterConfig::Http { endpoint, headers } => {
            // For OTLP HTTP, we'd use opentelemetry-otlp crate
            // This is a placeholder for the integration
            debug!(endpoint = %endpoint, "OTLP HTTP exporter configured (placeholder)");
            if let Some(h) = headers {
                debug!(headers = ?h.keys().collect::<Vec<_>>(), "OTLP headers configured");
            }

            let subscriber = Registry::default().with(filter);
            subscriber.try_init().map_err(|e| TracingError::InitializationFailed(e.to_string()))?;
        }
    }

    info!("Distributed tracing initialized successfully");
    Ok(())
}

/// Create a span for an operation
#[macro_export]
macro_rules! trace_span {
    ($name:expr) => {
        tracing::span!(tracing::Level::INFO, $name)
    };
    ($name:expr, $($key:tt => $value:tt),+) => {
        tracing::span!(tracing::Level::INFO, $name, $($key = $value),+)
    };
}

/// Instrument an async function with tracing
#[macro_export]
macro_rules! trace_async {
    ($name:expr, $func:expr) => {
        async {
            let span = tracing::span!(tracing::Level::INFO, $name);
            let _guard = span.enter();
            $func.await
        }
    };
}

/// Create a trace guard that records duration
pub struct TraceGuard {
    name: &'static str,
    start: std::time::Instant,
}

impl TraceGuard {
    /// Create a new trace guard
    pub fn new(name: &'static str) -> Self {
        let start = std::time::Instant::now();
        debug!(name = name, "Starting operation");
        Self { name, start }
    }

    /// Record an event within the span
    pub fn event(&self, message: &str) {
        debug!(name = %self.name, elapsed_ms = %self.start.elapsed().as_millis(), message = message, "Operation event");
    }

    /// Record an error within the span
    pub fn error(&self, error: &str) {
        error!(name = %self.name, elapsed_ms = %self.start.elapsed().as_millis(), error = error, "Operation error");
    }
}

impl Drop for TraceGuard {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        debug!(name = %self.name, elapsed_ms = %elapsed.as_millis(), "Operation completed");
    }
}

/// Start a traced operation
pub fn trace_operation(name: &'static str) -> TraceGuard {
    TraceGuard::new(name)
}

/// Log an event with trace context
pub fn log_trace_event(level: Level, message: &str, context: &TraceContext) {
    match level {
        Level::ERROR => error!(
            trace_id = %context.trace_id,
            span_id = %context.span_id,
            sampled = context.sampled,
            message = message
        ),
        Level::WARN => warn!(
            trace_id = %context.trace_id,
            span_id = %context.span_id,
            sampled = context.sampled,
            message = message
        ),
        Level::INFO => info!(
            trace_id = %context.trace_id,
            span_id = %context.span_id,
            sampled = context.sampled,
            message = message
        ),
        Level::DEBUG => debug!(
            trace_id = %context.trace_id,
            span_id = %context.span_id,
            sampled = context.sampled,
            message = message
        ),
        Level::TRACE => debug!(
            trace_id = %context.trace_id,
            span_id = %context.span_id,
            sampled = context.sampled,
            message = message
        ),
    }
}

/// Generate a random trace ID
pub fn generate_trace_id() -> String {
    use rand::Rng;
    let bytes: [u8; 16] = rand::thread_rng().gen();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Generate a random span ID
pub fn generate_span_id() -> String {
    use rand::Rng;
    let bytes: [u8; 8] = rand::thread_rng().gen();
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_context_creation() {
        let ctx = TraceContext::new("trace-123".to_string(), "span-456".to_string());
        assert_eq!(ctx.trace_id, "trace-123");
        assert_eq!(ctx.span_id, "span-456");
        assert!(ctx.sampled);
        assert!(ctx.parent_span_id.is_none());
    }

    #[test]
    fn test_trace_context_headers_roundtrip() {
        let mut ctx = TraceContext::new("trace-abc".to_string(), "span-def".to_string());
        ctx.sampled = false;
        ctx.baggage.insert("user_id".to_string(), "user-123".to_string());

        let headers = ctx.to_headers();
        let restored = TraceContext::from_headers(&headers);

        assert_eq!(restored.trace_id, "trace-abc");
        assert_eq!(restored.span_id, "span-def");
        assert!(!restored.sampled);
        assert_eq!(restored.baggage.get("user_id"), Some(&"user-123".to_string()));
    }

    #[test]
    fn test_span_attributes() {
        let attrs = SpanAttributes::new()
            .with_http_method("GET")
            .with_http_url("/api/users")
            .with_http_status(200)
            .with_peer_service("user-service");

        let map = attrs.as_map();
        assert_eq!(map.get("http.method"), Some(&"GET".to_string()));
        assert_eq!(map.get("http.url"), Some(&"/api/users".to_string()));
        assert_eq!(map.get("http.status_code"), Some(&"200".to_string()));
        assert_eq!(map.get("peer.service"), Some(&"user-service".to_string()));
    }

    #[test]
    fn test_generate_ids() {
        let trace_id = generate_trace_id();
        let span_id = generate_span_id();

        // Trace ID should be 32 hex chars (16 bytes)
        assert_eq!(trace_id.len(), 32);
        assert!(trace_id.chars().all(|c| c.is_ascii_hexdigit()));

        // Span ID should be 16 hex chars (8 bytes)
        assert_eq!(span_id.len(), 16);
        assert!(span_id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_trace_guard() {
        let guard = trace_operation("test_operation");
        guard.event("Processing started");
        // Guard drops here and logs completion
    }

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert_eq!(config.service_name, "nebula-service");
        assert_eq!(config.sample_rate, 1.0);
        assert!(config.include_local_spans);
        assert_eq!(config.max_buffered_spans, 10000);
    }

    #[test]
    fn test_exporter_config_serialization() {
        let console = ExporterConfig::Console { pretty: true, with_timestamps: false };
        let json = serde_json::to_string(&console).unwrap();
        assert!(json.contains("\"type\":\"console\""));
        assert!(json.contains("\"pretty\":true"));
    }
}
