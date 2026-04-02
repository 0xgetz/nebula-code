//! # Nebula Federated Learning Protocol
//!
//! A Rust implementation of federated learning protocols with support for:
//!
//! - **Core Types**: Client updates, aggregation rounds, model parameters, and configuration
//! - **Communication Protocol**: Async client-server interaction with message serialization
//! - **Aggregation Methods**: FedAvg, simple average, and weighted averaging
//! - **Privacy**: Differential privacy with Laplace and Gaussian mechanisms
//! - **Security**: Secure aggregation using Shamir's Secret Sharing and pairwise masking
//!
//! ## Example
//!
//! ```rust
//! use nebula_federated::types::*;
//! use nebula_federated::server::FederatedServerImpl;
//!
//! // Create server configuration
//! let config = FederatedConfig::new("my_job".to_string(), 100, 10)
//!     .with_client_fraction(0.1);
//!
//! // Initialize with a simple model
//! let initial_model = ModelParameters::new(vec![0.0; 1000]);
//!
//! // Create the server
//! let server = FederatedServerImpl::new(config, initial_model);
//! ```

pub mod client;
pub mod privacy;
pub mod protocol;
pub mod secure_aggregation;
pub mod server;
pub mod types;

// Re-export commonly used types
pub use client::{FederatedClientImpl, MockClient};
pub use privacy::{
    clip_gradient_by_norm, DifferentialPrivacyConfig, NoiseMechanism, PrivacyAccountant,
};
pub use protocol::{
    ClientCapabilities, ClientMessage, ComputeResources, FederatedClient, FederatedServer,
    JsonProtocol, ProtocolHandler, ServerMessage, UpdateConfig,
};
pub use secure_aggregation::{
    MaskedUpdate, SecureAggregator, SecureAggregationState, SecureClient, SecretShare,
    ShamirParams, ShamirSecretSharing,
};
pub use server::FederatedServerImpl;
pub use types::{
    AggregationConfig, AggregationMethod, AggregationRound, ClientId, ClientMetadata,
    ClientUpdate, FederatedConfig, FederatedError, ModelConfig, ModelParameters,
    ParameterMetadata, PrivacyConfig, RoundId, RoundStatus,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if the library is properly initialized
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "0.1.0");
    }

    #[tokio::test]
    async fn test_basic_workflow() {
        // Create server
        let config = FederatedConfig::new("test".to_string(), 10, 5);
        let model = ModelParameters::new(vec![0.0; 10]);
        let server = FederatedServerImpl::new(config, model);

        // Start server
        server.start().await.unwrap();

        // Register clients
        for i in 0..5 {
            server
                .register_client(format!("client_{}", i), ClientCapabilities::default())
                .await
                .unwrap();
        }

        assert_eq!(server.num_active_clients().await, 5);

        // Start round
        let round_id = server.start_round().await.unwrap();

        // Simulate client updates
        for i in 0..3 {
            let params = ModelParameters::new(vec![1.0; 10]);
            let update = ClientUpdate::new(format!("client_{}", i), round_id, params, 100);
            server.process_update(update).await.unwrap();
        }

        // Check round completion
        let status = server.get_round_status(round_id).await.unwrap();
        assert_eq!(status, RoundStatus::Completed);

        // Verify global model was updated
        let global_model = server.get_global_model().await.unwrap();
        assert_eq!(global_model.values, vec![1.0; 10]);
    }

    #[test]
    fn test_privacy_integration() {
        // Create privacy configuration with Gaussian mechanism
        let config = DifferentialPrivacyConfig::gaussian(1.0, 1e-5, 1.0);
        let mut accountant = PrivacyAccountant::new(config);

        // Create a model update
        let update = ModelParameters::new(vec![2.0; 100]);

        // Clip the update
        let clipped = accountant.clip_update(&update);
        let norm: f64 = clipped.values.iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!(norm <= 1.0 + 1e-10);

        // Add noise
        let _noisy = accountant.add_noise(clipped, 10);

        // Process a full round
        let update2 = ModelParameters::new(vec![3.0; 50]);
        let result = accountant.process_round(update2, 5);
        assert_eq!(result.values.len(), 50);
        assert_eq!(accountant.rounds_completed(), 1);
    }

    #[test]
    fn test_secure_aggregation_integration() {
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        // Setup secure aggregation with 5 clients, threshold 3, model dim 10
        let mut aggregator = SecureAggregator::new(5, 3, 10).unwrap();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        aggregator.state.generate_pairwise_seeds(&mut rng);

        // Simulate clients with local updates
        let original_updates: Vec<Vec<f64>> = (0..5)
            .map(|i| (0..10).map(|j| (i + j) as f64 * 0.1).collect())
            .collect();

        // Each client masks their update
        let masked_updates: Vec<Vec<f64>> = original_updates
            .iter()
            .enumerate()
            .map(|(i, weights)| {
                let client = SecureClient::new(i, 10);
                client
                    .generate_masked_update(weights, &aggregator)
                    .unwrap()
                    .masked_weights
            })
            .collect();

        // Server aggregates
        let aggregated = aggregator.aggregate_masked_updates(&masked_updates).unwrap();

        // Expected sum
        let expected: Vec<f64> = (0..10)
            .map(|j| original_updates.iter().map(|u| u[j]).sum())
            .collect();

        // Verify correctness
        for (a, e) in aggregated.iter().zip(expected.iter()) {
            assert!((a - e).abs() < 1e-6);
        }
    }

    #[test]
    fn test_shamir_integration() {
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        let mut rng = ChaCha20Rng::seed_from_u64(123);
        let secret = 42.0;
        let params = ShamirParams::new(5, 3).unwrap();

        // Generate shares
        let shares = ShamirSecretSharing::generate_shares(secret, &params, &mut rng);
        assert_eq!(shares.len(), 5);

        // Reconstruct with threshold shares
        let subset = &shares[0..3];
        let reconstructed = ShamirSecretSharing::reconstruct_secret(subset).unwrap();
        assert!((reconstructed - secret).abs() < 1e-6);
    }
}
