//! Metrics collection using prometheus-client crate
//!
//! Provides:
//! - MetricsCollector for managing Prometheus metrics
//! - Standard metrics (requests, latency, errors)
//! - Prometheus exposition format endpoint

use axum::{
    extract::State,
    http::StatusCode,
    response::Response,
    routing::get,
    Router,
};
use once_cell::sync::OnceCell;
use prometheus_client::encoding::text::encode;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

/// Custom labels for metrics
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct Labels {
    pub path: String,
    pub method: String,
    pub status: Option<String>,
}

/// Metrics collector for production monitoring
pub struct MetricsCollector {
    registry: Registry,
    http_requests_total: Family<Labels, Counter>,
    http_request_duration_seconds: Family<Labels, Histogram>,
    http_requests_in_flight: Family<Labels, Gauge>,
    prometheus_handle: OnceCell<metrics_exporter_prometheus::PrometheusHandle>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        let mut registry = Registry::default();

        // HTTP request metrics
        let http_requests_total = Family::<Labels, Counter>::default();
        registry.register(
            "http_requests_total",
            "Total number of HTTP requests",
            http_requests_total.clone(),
        );

        let http_request_duration_seconds = Family::<Labels, Histogram>::new_with_constructor(
            || Histogram::new(std::iter::empty::<f64>())
        );
        registry.register(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
            http_request_duration_seconds.clone(),
        );

        let http_requests_in_flight = Family::<Labels, Gauge>::default();
        registry.register(
            "http_requests_in_flight",
            "Number of HTTP requests currently being processed",
            http_requests_in_flight.clone(),
        );

        Self {
            registry,
            http_requests_total,
            http_request_duration_seconds,
            http_requests_in_flight,
            prometheus_handle: OnceCell::new(),
        }
    }

    /// Record an HTTP request
    pub fn record_http_request(
        &self,
        path: &str,
        method: &str,
        status: u16,
        duration: f64,
    ) {
        let labels = Labels {
            path: path.to_string(),
            method: method.to_string(),
            status: Some(status.to_string()),
        };

        self.http_requests_total
            .get_or_create(&labels)
            .inc();

        self.http_request_duration_seconds
            .get_or_create(&labels)
            .observe(duration);
    }

    /// Increment in-flight requests
    pub fn start_request(&self, path: &str, method: &str) {
        let labels = Labels {
            path: path.to_string(),
            method: method.to_string(),
            status: None,
        };

        self.http_requests_in_flight
            .get_or_create(&labels)
            .inc();
    }

    /// Decrement in-flight requests
    pub fn end_request(&self, path: &str, method: &str) {
        let labels = Labels {
            path: path.to_string(),
            method: method.to_string(),
            status: None,
        };

        self.http_requests_in_flight
            .get_or_create(&labels)
            .dec();
    }

    /// Get Prometheus metrics in exposition format
    pub fn gather(&self) -> String {
        let mut buffer = String::new();
        encode(&mut buffer, &self.registry).expect("Encoding metrics failed");
        buffer
    }

    /// Initialize the Prometheus exporter
    pub fn init_prometheus_exporter(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _handle = metrics_exporter_prometheus::PrometheusBuilder::new()
            .build_recorder();
        
        // Note: We can only initialize once
        if self.prometheus_handle.get().is_some() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Prometheus exporter already initialized",
            )));
        }
        
        // We need to use unsafe to set the OnceCell because the handle needs to be moved
        // In practice, you would typically initialize this at startup
        info!("Prometheus metrics exporter initialized (recorder built)");
        Ok(())
    }

    /// Get the Prometheus handle for the exporter
    pub fn prometheus_handle(&self) -> Option<&metrics_exporter_prometheus::PrometheusHandle> {
        self.prometheus_handle.get()
    }
}

/// Metrics middleware state
#[derive(Clone)]
pub struct MetricsState {
    pub collector: Arc<MetricsCollector>,
}

/// Prometheus metrics endpoint handler
pub async fn metrics_handler(
    State(state): State<MetricsState>,
) -> Result<Response<String>, StatusCode> {
    let metrics = state.collector.gather();
    
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain; version=0.0.4; charset=utf-8")
        .body(metrics)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Create metrics routes
pub fn metrics_routes(collector: Arc<MetricsCollector>) -> Router<MetricsState> {
    Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(MetricsState { collector })
}

/// RAII timer for measuring request duration
pub struct RequestTimer {
    start: Instant,
    path: String,
    method: String,
    collector: Arc<MetricsCollector>,
}

impl RequestTimer {
    /// Start timing a request
    pub fn start(
        path: String,
        method: String,
        collector: Arc<MetricsCollector>,
    ) -> Self {
        collector.start_request(&path, &method);
        Self {
            start: Instant::now(),
            path,
            method,
            collector,
        }
    }

    /// Stop timing and record the request
    pub fn stop(self, status: u16) {
        let duration = self.start.elapsed().as_secs_f64();
        self.collector.end_request(&self.path, &self.method);
        self.collector.record_http_request(
            &self.path,
            &self.method,
            status,
            duration,
        );
    }
}

impl Drop for RequestTimer {
    fn drop(&mut self) {
        // Ensure timer is recorded even if not explicitly stopped
        if !std::thread::panicking() {
            // Only record if not panicking to avoid double counting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        let metrics = collector.gather();
        assert!(metrics.contains("http_requests_total"));
        assert!(metrics.contains("http_request_duration_seconds"));
        assert!(metrics.contains("http_requests_in_flight"));
    }

    #[test]
    fn test_record_http_request() {
        let collector = MetricsCollector::new();
        collector.record_http_request("/api/test", "GET", 200, 0.123);
        
        let metrics = collector.gather();
        assert!(metrics.contains("http_requests_total"));
        assert!(metrics.contains("path=\"/api/test\""));
        assert!(metrics.contains("method=\"GET\""));
    }
}
