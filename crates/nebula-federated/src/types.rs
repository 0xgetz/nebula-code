//! Core types for federated learning protocol

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a client in the federated network
pub type ClientId = String;

/// Unique identifier for an aggregation round
pub type RoundId = u64;

/// Represents model parameters as a vector of f64 values
/// This is a flattened representation for simplicity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelParameters {
    /// Flattened parameter vector
    pub values: Vec<f64>,
    /// Metadata about the parameter structure
    pub metadata: ParameterMetadata,
}

impl ModelParameters {
    /// Create new model parameters from a vector
    pub fn new(values: Vec<f64>) -> Self {
        Self {
            values,
            metadata: ParameterMetadata::default(),
        }
    }

    /// Create with metadata
    pub fn with_metadata(values: Vec<f64>, metadata: ParameterMetadata) -> Self {
        Self { values, metadata }
    }

    /// Get the number of parameters
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if parameters are empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Element-wise addition with another parameter vector
    pub fn add(&self, other: &Self) -> Result<Self, FederatedError> {
        if self.len() != other.len() {
            return Err(FederatedError::ParameterMismatch {
                expected: self.len(),
                got: other.len(),
            });
        }
        let values = self.values.iter().zip(other.values.iter()).map(|(a, b)| a + b).collect();
        Ok(Self {
            values,
            metadata: self.metadata.clone(),
        })
    }

    /// Scalar multiplication
    pub fn scale(&self, scalar: f64) -> Self {
        Self {
            values: self.values.iter().map(|v| v * scalar).collect(),
            metadata: self.metadata.clone(),
        }
    }
}

/// Metadata about model parameter structure
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ParameterMetadata {
    /// Name of the model
    pub model_name: Option<String>,
    /// Layer shapes (for neural networks)
    pub layer_shapes: Vec<Vec<usize>>,
    /// Parameter names
    pub param_names: Vec<String>,
    /// Creation timestamp
    pub created_at: Option<u64>,
}

/// Represents an update from a client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientUpdate {
    /// Client identifier
    pub client_id: ClientId,
    /// Round this update belongs to
    pub round_id: RoundId,
    /// Model parameter updates (gradients or weights)
    pub parameters: ModelParameters,
    /// Number of samples used for this update
    pub num_samples: usize,
    /// Quality metrics (accuracy, loss, etc.)
    pub metrics: HashMap<String, f64>,
    /// Timestamp of the update
    pub timestamp: u64,
    /// Optional metadata
    pub metadata: ClientMetadata,
}

impl ClientUpdate {
    /// Create a new client update
    pub fn new(
        client_id: ClientId,
        round_id: RoundId,
        parameters: ModelParameters,
        num_samples: usize,
    ) -> Self {
        Self {
            client_id,
            round_id,
            parameters,
            num_samples,
            metrics: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata: ClientMetadata::default(),
        }
    }

    /// Add a metric to the update
    pub fn with_metric(mut self, key: String, value: f64) -> Self {
        self.metrics.insert(key, value);
        self
    }
}

/// Metadata about a client
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientMetadata {
    /// Client hardware info
    pub device_type: Option<String>,
    /// Network conditions
    pub bandwidth_mbps: Option<f64>,
    /// Geographic region
    pub region: Option<String>,
}

/// Represents a single aggregation round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationRound {
    /// Round identifier
    pub round_id: RoundId,
    /// Current global model
    pub global_model: ModelParameters,
    /// Updates received from clients
    pub client_updates: Vec<ClientUpdate>,
    /// Status of the round
    pub status: RoundStatus,
    /// Start timestamp
    pub started_at: u64,
    /// Completion timestamp (if finished)
    pub completed_at: Option<u64>,
    /// Aggregation configuration for this round
    pub config: AggregationConfig,
}

impl AggregationRound {
    /// Create a new aggregation round
    pub fn new(round_id: RoundId, global_model: ModelParameters, config: AggregationConfig) -> Self {
        Self {
            round_id,
            global_model,
            client_updates: Vec::new(),
            status: RoundStatus::Active,
            started_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            completed_at: None,
            config,
        }
    }

    /// Add a client update to the round
    pub fn add_update(&mut self, update: ClientUpdate) {
        self.client_updates.push(update);
    }

    /// Get number of client updates
    pub fn num_updates(&self) -> usize {
        self.client_updates.len()
    }

    /// Mark round as complete
    pub fn complete(&mut self) {
        self.status = RoundStatus::Completed;
        self.completed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        );
    }
}

/// Status of an aggregation round
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoundStatus {
    /// Round is active and accepting updates
    Active,
    /// Round is complete
    Completed,
    /// Round failed
    Failed,
    /// Round was cancelled
    Cancelled,
}

/// Configuration for aggregation in a round
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationConfig {
    /// Minimum number of clients required
    pub min_clients: usize,
    /// Maximum number of clients to accept
    pub max_clients: usize,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Aggregation method
    pub method: AggregationMethod,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self {
            min_clients: 3,
            max_clients: 100,
            timeout_ms: 60000,
            method: AggregationMethod::FedAvg,
        }
    }
}

/// Aggregation methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationMethod {
    /// Federated Averaging (weighted by sample count)
    FedAvg,
    /// Simple average (equal weights)
    SimpleAverage,
    /// Weighted average with custom weights
    WeightedAverage,
}

/// Configuration for the federated learning system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederatedConfig {
    /// Unique identifier for this federated job
    pub job_id: String,
    /// Total number of expected clients
    pub total_clients: usize,
    /// Number of communication rounds
    pub num_rounds: usize,
    /// Fraction of clients to sample per round
    pub client_fraction: f64,
    /// Aggregation configuration
    pub aggregation: AggregationConfig,
    /// Privacy configuration
    pub privacy: PrivacyConfig,
    /// Model configuration
    pub model: ModelConfig,
}

impl FederatedConfig {
    /// Create a new federated configuration
    pub fn new(job_id: String, total_clients: usize, num_rounds: usize) -> Self {
        Self {
            job_id,
            total_clients,
            num_rounds,
            client_fraction: 0.1,
            aggregation: AggregationConfig::default(),
            privacy: PrivacyConfig::default(),
            model: ModelConfig::default(),
        }
    }

    /// Set client fraction
    pub fn with_client_fraction(mut self, fraction: f64) -> Self {
        self.client_fraction = fraction.clamp(0.01, 1.0);
        self
    }

    /// Set privacy configuration
    pub fn with_privacy(mut self, privacy: PrivacyConfig) -> Self {
        self.privacy = privacy;
        self
    }
}

/// Privacy configuration for differential privacy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Whether differential privacy is enabled
    pub enabled: bool,
    /// Privacy budget (epsilon)
    pub epsilon: f64,
    /// Delta for (epsilon, delta)-DP
    pub delta: f64,
    /// Noise multiplier
    pub noise_multiplier: f64,
    /// Clipping norm for gradients
    pub clipping_norm: f64,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            epsilon: 1.0,
            delta: 1e-5,
            noise_multiplier: 1.0,
            clipping_norm: 1.0,
        }
    }
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model architecture name
    pub architecture: String,
    /// Number of parameters
    pub num_params: usize,
    /// Learning rate
    pub learning_rate: f64,
    /// Batch size
    pub batch_size: usize,
    /// Number of local epochs
    pub local_epochs: usize,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            architecture: "mlp".to_string(),
            num_params: 1000,
            learning_rate: 0.01,
            batch_size: 32,
            local_epochs: 5,
        }
    }
}

/// Error types for federated learning
#[derive(Debug, thiserror::Error)]
pub enum FederatedError {
    #[error("Parameter mismatch: expected {expected}, got {got}")]
    ParameterMismatch { expected: usize, got: usize },

    #[error("Client {client_id} not found in round {round_id}")]
    ClientNotFound { client_id: ClientId, round_id: RoundId },

    #[error("Round {round_id} is not active")]
    RoundNotActive { round_id: RoundId },

    #[error("Insufficient clients: need {needed}, have {available}")]
    InsufficientClients { needed: usize, available: usize },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Privacy budget exceeded")]
    PrivacyBudgetExceeded,

    #[error("Aggregation failed: {0}")]
    AggregationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_parameters_operations() {
        let p1 = ModelParameters::new(vec![1.0, 2.0, 3.0]);
        let p2 = ModelParameters::new(vec![4.0, 5.0, 6.0]);

        let sum = p1.add(&p2).unwrap();
        assert_eq!(sum.values, vec![5.0, 7.0, 9.0]);

        let scaled = p1.scale(2.0);
        assert_eq!(scaled.values, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_client_update_creation() {
        let params = ModelParameters::new(vec![1.0, 2.0, 3.0]);
        let update = ClientUpdate::new("client_1".to_string(), 1, params, 100)
            .with_metric("accuracy".to_string(), 0.95);

        assert_eq!(update.client_id, "client_1");
        assert_eq!(update.num_samples, 100);
        assert_eq!(*update.metrics.get("accuracy").unwrap(), 0.95);
    }

    #[test]
    fn test_aggregation_round() {
        let model = ModelParameters::new(vec![0.0; 10]);
        let mut round = AggregationRound::new(1, model, AggregationConfig::default());

        assert_eq!(round.status, RoundStatus::Active);
        assert_eq!(round.num_updates(), 0);

        let update = ClientUpdate::new("client_1".to_string(), 1, ModelParameters::new(vec![1.0; 10]), 50);
        round.add_update(update);

        assert_eq!(round.num_updates(), 1);
    }
}
