//! Alerting system for production monitoring
//!
//! Provides alert management with severity levels, notification channels,
//! and alert routing based on conditions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Low priority, informational only
    Low,
    /// Medium priority, requires attention
    Medium,
    /// High priority, immediate attention needed
    High,
    /// Critical, system down or data loss
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Low => write!(f, "low"),
            Severity::Medium => write!(f, "medium"),
            Severity::High => write!(f, "high"),
            Severity::Critical => write!(f, "critical"),
        }
    }
}

/// Alert status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertStatus {
    /// Alert is active and firing
    Firing,
    /// Alert has been acknowledged
    Acknowledged,
    /// Alert has been resolved
    Resolved,
}

impl std::fmt::Display for AlertStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertStatus::Firing => write!(f, "firing"),
            AlertStatus::Acknowledged => write!(f, "acknowledged"),
            AlertStatus::Resolved => write!(f, "resolved"),
        }
    }
}

/// Notification channel types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NotificationChannel {
    /// Send to a webhook URL
    Webhook {
        url: String,
        headers: Option<HashMap<String, String>>,
    },
    /// Send to stdout (for development)
    Console { include_timestamp: bool },
    /// Send to a file
    File { path: String },
    /// Send via email (placeholder for integration)
    Email { recipients: Vec<String> },
}

/// Alert definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique alert identifier
    pub id: String,
    /// Human-readable alert name
    pub name: String,
    /// Alert description
    pub description: String,
    /// Severity level
    pub severity: Severity,
    /// Current status
    pub status: AlertStatus,
    /// Service or component this alert is for
    pub service: String,
    /// Additional metadata
    pub labels: HashMap<String, String>,
    /// When the alert was created
    pub created_at: DateTime<Utc>,
    /// When the alert was last updated
    pub updated_at: DateTime<Utc>,
    /// When the alert was resolved (if applicable)
    pub resolved_at: Option<DateTime<Utc>>,
    /// Value that triggered the alert
    pub value: Option<f64>,
    /// Threshold that was exceeded
    pub threshold: Option<f64>,
}

impl Alert {
    /// Create a new alert
    pub fn new(id: String, name: String, description: String, severity: Severity, service: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            name,
            description,
            severity,
            status: AlertStatus::Firing,
            service,
            labels: HashMap::new(),
            created_at: now,
            updated_at: now,
            resolved_at: None,
            value: None,
            threshold: None,
        }
    }

    /// Add a label to the alert
    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    /// Set the value and threshold
    pub fn with_threshold(mut self, value: f64, threshold: f64) -> Self {
        self.value = Some(value);
        self.threshold = Some(threshold);
        self
    }

    /// Acknowledge the alert
    pub fn acknowledge(&mut self) {
        self.status = AlertStatus::Acknowledged;
        self.updated_at = Utc::now();
    }

    /// Resolve the alert
    pub fn resolve(&mut self) {
        self.status = AlertStatus::Resolved;
        self.resolved_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Unique rule identifier
    pub id: String,
    /// Rule name
    pub name: String,
    /// Metric name to evaluate
    pub metric: String,
    /// Comparison operator
    pub operator: AlertOperator,
    /// Threshold value
    pub threshold: f64,
    /// Severity to assign when triggered
    pub severity: Severity,
    /// How long the condition must be true before alerting (seconds)
    pub for_duration_secs: u64,
    /// Notification channels to use
    pub channels: Vec<String>,
    /// Additional labels to add to alerts
    pub labels: HashMap<String, String>,
}

/// Comparison operators for alert rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertOperator {
    /// Greater than
    Gt,
    /// Greater than or equal
    Gte,
    /// Less than
    Lt,
    /// Less than or equal
    Lte,
    /// Equal
    Eq,
    /// Not equal
    Ne,
}

impl AlertOperator {
    /// Evaluate the comparison
    pub fn eval(&self, value: f64, threshold: f64) -> bool {
        match self {
            AlertOperator::Gt => value > threshold,
            AlertOperator::Gte => value >= threshold,
            AlertOperator::Lt => value < threshold,
            AlertOperator::Lte => value <= threshold,
            AlertOperator::Eq => value == threshold,
            AlertOperator::Ne => value != threshold,
        }
    }
}

/// Errors that can occur in alerting
#[derive(Debug, Error)]
pub enum AlertError {
    #[error("Alert not found: {0}")]
    NotFound(String),
    #[error("Channel not found: {0}")]
    ChannelNotFound(String),
    #[error("Failed to send notification: {0}")]
    NotificationFailed(String),
    #[error("Invalid alert configuration: {0}")]
    InvalidConfig(String),
}

/// Alert manager for handling alerts and notifications
pub struct AlertManager {
    /// Registered notification channels
    channels: Arc<RwLock<HashMap<String, NotificationChannel>>>,
    /// Active and historical alerts
    alerts: Arc<RwLock<HashMap<String, Alert>>>,
    /// Alert rules
    rules: Arc<RwLock<HashMap<String, AlertRule>>>,
    /// Notification callback for custom handling
    notification_callback: Option<Arc<dyn Fn(&Alert, &NotificationChannel) -> Result<(), AlertError> + Send + Sync>>,
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            alerts: Arc::new(RwLock::new(HashMap::new())),
            rules: Arc::new(RwLock::new(HashMap::new())),
            notification_callback: None,
        }
    }

    /// Set a custom notification callback
    pub fn with_notification_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Alert, &NotificationChannel) -> Result<(), AlertError> + Send + Sync + 'static,
    {
        self.notification_callback = Some(Arc::new(callback));
        self
    }

    /// Register a notification channel
    pub async fn add_channel(&self, name: String, channel: NotificationChannel) {
        let mut channels = self.channels.write().await;
        channels.insert(name, channel);
    }

    /// Register an alert rule
    pub async fn add_rule(&self, rule: AlertRule) {
        let mut rules = self.rules.write().await;
        rules.insert(rule.id.clone(), rule);
    }

    /// Create and fire a new alert
    pub async fn fire_alert(&self, alert: Alert) -> Result<(), AlertError> {
        let alert_id = alert.id.clone();

        // Store the alert
        {
            let mut alerts = self.alerts.write().await;
            alerts.insert(alert_id.clone(), alert);
        }

        // Send notifications
        self.send_notifications(&alert_id).await?;

        Ok(())
    }

    /// Create an alert with the given parameters
    pub async fn create_alert(
        &self,
        id: String,
        name: String,
        description: String,
        severity: Severity,
        service: String,
    ) -> Result<(), AlertError> {
        let alert = Alert::new(id, name, description, severity, service);
        self.fire_alert(alert).await
    }

    /// Acknowledge an alert
    pub async fn acknowledge_alert(&self, alert_id: &str) -> Result<(), AlertError> {
        let mut alerts = self.alerts.write().await;
        let alert = alerts.get_mut(alert_id).ok_or_else(|| AlertError::NotFound(alert_id.to_string()))?;
        alert.acknowledge();
        Ok(())
    }

    /// Resolve an alert
    pub async fn resolve_alert(&self, alert_id: &str) -> Result<(), AlertError> {
        let mut alerts = self.alerts.write().await;
        let alert = alerts.get_mut(alert_id).ok_or_else(|| AlertError::NotFound(alert_id.to_string()))?;
        alert.resolve();
        Ok(())
    }

    /// Get an alert by ID
    pub async fn get_alert(&self, alert_id: &str) -> Option<Alert> {
        let alerts = self.alerts.read().await;
        alerts.get(alert_id).cloned()
    }

    /// Get all alerts
    pub async fn list_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.values().cloned().collect()
    }

    /// Get active (firing) alerts
    pub async fn active_alerts(&self) -> Vec<Alert> {
        let alerts = self.alerts.read().await;
        alerts.values().filter(|a| a.status == AlertStatus::Firing).cloned().collect()
    }

    /// Evaluate a metric value against all rules and fire alerts if needed
    pub async fn evaluate_metric(&self, metric_name: &str, value: f64) -> Result<Vec<String>, AlertError> {
        let rules = self.rules.read().await;
        let mut fired_alerts = Vec::new();

        for rule in rules.values() {
            if rule.metric == metric_name && rule.operator.eval(value, rule.threshold) {
                let alert_id = format!("{}_{}", rule.id, chrono::Utc::now().timestamp());
                let mut alert = Alert::new(
                    alert_id.clone(),
                    rule.name.clone(),
                    format!("Metric {} is {} {}", metric_name, rule.operator.as_str(), rule.threshold),
                    rule.severity,
                    metric_name.to_string(),
                )
                .with_threshold(value, rule.threshold);

                // Add rule labels
                for (k, v) in &rule.labels {
                    alert = alert.with_label(k.clone(), v.clone());
                }

                self.fire_alert(alert).await?;
                fired_alerts.push(alert_id);
            }
        }

        Ok(fired_alerts)
    }

    /// Get alert statistics
    pub async fn stats(&self) -> AlertStats {
        let alerts = self.alerts.read().await;
        let mut stats = AlertStats::default();

        for alert in alerts.values() {
            match alert.status {
                AlertStatus::Firing => stats.firing += 1,
                AlertStatus::Acknowledged => stats.acknowledged += 1,
                AlertStatus::Resolved => stats.resolved += 1,
            }

            match alert.severity {
                Severity::Low => stats.by_severity.low += 1,
                Severity::Medium => stats.by_severity.medium += 1,
                Severity::High => stats.by_severity.high += 1,
                Severity::Critical => stats.by_severity.critical += 1,
            }
        }

        stats.total = alerts.len();
        stats
    }

    /// Send notifications for an alert
    async fn send_notifications(&self, alert_id: &str) -> Result<(), AlertError> {
        let alert = {
            let alerts = self.alerts.read().await;
            alerts.get(alert_id).cloned().ok_or_else(|| AlertError::NotFound(alert_id.to_string()))?
        };

        // For now, we'll just log the alert
        // In a real implementation, this would send to the configured channels
        match alert.severity {
            Severity::Critical => {
                error!(alert_id = %alert.id, name = %alert.name, severity = %alert.severity, "Alert fired");
            }
            Severity::High => {
                warn!(alert_id = %alert.id, name = %alert.name, severity = %alert.severity, "Alert fired");
            }
            _ => {
                info!(alert_id = %alert.id, name = %alert.name, severity = %alert.severity, "Alert fired");
            }
        }

        // If a custom callback is configured, use it
        if let Some(callback) = &self.notification_callback {
            let channel = NotificationChannel::Console { include_timestamp: true };
            callback(&alert, &channel)?;
        }

        Ok(())
    }
}

/// Statistics about alerts
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AlertStats {
    /// Total number of alerts
    pub total: usize,
    /// Number of firing alerts
    pub firing: usize,
    /// Number of acknowledged alerts
    pub acknowledged: usize,
    /// Number of resolved alerts
    pub resolved: usize,
    /// Alerts grouped by severity
    pub by_severity: SeverityCounts,
}

/// Counts of alerts by severity
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
    pub critical: usize,
}

impl AlertOperator {
    /// Get string representation of operator
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertOperator::Gt => ">",
            AlertOperator::Gte => ">=",
            AlertOperator::Lt => "<",
            AlertOperator::Lte => "<=",
            AlertOperator::Eq => "==",
            AlertOperator::Ne => "!=",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_alert_creation() {
        let alert = Alert::new(
            "test-1".to_string(),
            "High CPU".to_string(),
            "CPU usage is too high".to_string(),
            Severity::High,
            "system".to_string(),
        );

        assert_eq!(alert.id, "test-1");
        assert_eq!(alert.status, AlertStatus::Firing);
        assert_eq!(alert.severity, Severity::High);
    }

    #[tokio::test]
    async fn test_alert_acknowledge() {
        let mut alert = Alert::new(
            "test-2".to_string(),
            "High CPU".to_string(),
            "CPU usage is too high".to_string(),
            Severity::High,
            "system".to_string(),
        );

        alert.acknowledge();
        assert_eq!(alert.status, AlertStatus::Acknowledged);
    }

    #[tokio::test]
    async fn test_alert_resolve() {
        let mut alert = Alert::new(
            "test-3".to_string(),
            "High CPU".to_string(),
            "CPU usage is too high".to_string(),
            Severity::High,
            "system".to_string(),
        );

        alert.resolve();
        assert_eq!(alert.status, AlertStatus::Resolved);
        assert!(alert.resolved_at.is_some());
    }

    #[tokio::test]
    async fn test_operator_evaluation() {
        assert!(AlertOperator::Gt.eval(10.0, 5.0));
        assert!(!AlertOperator::Gt.eval(5.0, 5.0));
        assert!(AlertOperator::Gte.eval(5.0, 5.0));
        assert!(AlertOperator::Lt.eval(3.0, 5.0));
        assert!(!AlertOperator::Lt.eval(5.0, 5.0));
        assert!(AlertOperator::Eq.eval(5.0, 5.0));
        assert!(AlertOperator::Ne.eval(5.0, 3.0));
    }

    #[tokio::test]
    async fn test_alert_manager_basic() {
        let manager = AlertManager::new();

        // Add a channel
        manager
            .add_channel("console".to_string(), NotificationChannel::Console { include_timestamp: true })
            .await;

        // Create an alert
        manager
            .create_alert(
                "alert-1".to_string(),
                "Test Alert".to_string(),
                "A test alert".to_string(),
                Severity::Medium,
                "test-service".to_string(),
            )
            .await
            .unwrap();

        // Get the alert
        let alert = manager.get_alert("alert-1").await;
        assert!(alert.is_some());
        let alert = alert.unwrap();
        assert_eq!(alert.name, "Test Alert");
    }

    #[tokio::test]
    async fn test_alert_manager_stats() {
        let manager = AlertManager::new();

        // Create alerts with different severities
        manager
            .create_alert("a1".to_string(), "Low".to_string(), "Low alert".to_string(), Severity::Low, "svc".to_string())
            .await
            .unwrap();

        manager
            .create_alert("a2".to_string(), "Medium".to_string(), "Medium alert".to_string(), Severity::Medium, "svc".to_string())
            .await
            .unwrap();

        manager
            .create_alert("a3".to_string(), "High".to_string(), "High alert".to_string(), Severity::High, "svc".to_string())
            .await
            .unwrap();

        let stats = manager.stats().await;
        assert_eq!(stats.total, 3);
        assert_eq!(stats.firing, 3);
        assert_eq!(stats.by_severity.low, 1);
        assert_eq!(stats.by_severity.medium, 1);
        assert_eq!(stats.by_severity.high, 1);
    }

    #[tokio::test]
    async fn test_evaluate_metric() {
        let manager = AlertManager::new();

        // Add a rule
        let rule = AlertRule {
            id: "cpu-high".to_string(),
            name: "High CPU Usage".to_string(),
            metric: "cpu_usage".to_string(),
            operator: AlertOperator::Gt,
            threshold: 80.0,
            severity: Severity::High,
            for_duration_secs: 60,
            channels: vec!["console".to_string()],
            labels: HashMap::new(),
        };
        manager.add_rule(rule).await;

        // Evaluate above threshold
        let fired = manager.evaluate_metric("cpu_usage", 90.0).await.unwrap();
        assert_eq!(fired.len(), 1);

        // Evaluate below threshold
        let fired = manager.evaluate_metric("cpu_usage", 50.0).await.unwrap();
        assert!(fired.is_empty());
    }

    #[tokio::test]
    async fn test_active_alerts() {
        let manager = AlertManager::new();

        manager
            .create_alert("a1".to_string(), "Alert 1".to_string(), "Test".to_string(), Severity::Low, "svc".to_string())
            .await
            .unwrap();

        manager
            .create_alert("a2".to_string(), "Alert 2".to_string(), "Test".to_string(), Severity::Medium, "svc".to_string())
            .await
            .unwrap();

        // Acknowledge one
        manager.acknowledge_alert("a1").await.unwrap();

        let active = manager.active_alerts().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "a2");
    }

    #[tokio::test]
    async fn test_list_alerts() {
        let manager = AlertManager::new();

        manager
            .create_alert("a1".to_string(), "Alert 1".to_string(), "Test".to_string(), Severity::Low, "svc".to_string())
            .await
            .unwrap();

        let all = manager.list_alerts().await;
        assert_eq!(all.len(), 1);
    }
}
