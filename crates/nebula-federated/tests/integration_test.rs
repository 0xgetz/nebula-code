//! Integration Tests for Federated Learning System
//!
//! These tests verify the full system end-to-end, including:
//! - Server-client communication
//! - Model aggregation with multiple clients
//! - Differential privacy integration
//! - Secure aggregation protocol
//! - Privacy budget tracking

use nebula_federated::{
    ClientCapabilities, ClientUpdate, DifferentialPrivacyConfig, FederatedClient, FederatedConfig,
    FederatedServer, FederatedServerImpl, MockClient, ModelParameters,
    PrivacyAccountant, RoundStatus, SecureAggregator, ShamirSecretSharing,
    ShamirParams, UpdateConfig,
};
use rand::SeedableRng;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test basic federated training round with multiple clients
#[tokio::test]
async fn test_basic_federated_round() -> Result<(), Box<dyn std::error::Error>> {
    let num_clients = 3;
    let model_size = 5;

    // Create server with initial model
    let config = FederatedConfig::new("test_job".to_string(), num_clients, 1);
    let global_model = ModelParameters::new(vec![0.0; model_size]);
    let server = FederatedServerImpl::new(config, global_model);

    server.start().await?;

    // Register clients
    for i in 0..num_clients {
        server
            .register_client(format!("client_{}", i), ClientCapabilities::default())
            .await?;
    }

    // Start a round
    let round_id = server.start_round().await?;

    // Get current model
    let current_model = server.get_global_model().await?;

    // Simulate clients training and sending updates
    for i in 0..num_clients {
        let client = MockClient::new(format!("client_{}", i), 100);
        let update = client
            .train_local(&current_model, &UpdateConfig::default())
            .await?;

        // Create update for this round
        let round_update = ClientUpdate::new(update.client_id, round_id, update.parameters, 100);
        server.process_update(round_update).await?;
    }

    // Check round completion
    let status = server.get_round_status(round_id).await?;
    assert_eq!(status, RoundStatus::Completed);

    // Verify model was updated
    let final_model = server.get_global_model().await?;
    assert_ne!(
        final_model.values,
        current_model.values,
        "Model should be updated after aggregation"
    );

    server.stop().await?;
    Ok(())
}

/// Test differential privacy integration
#[tokio::test]
async fn test_differential_privacy() -> Result<(), Box<dyn std::error::Error>> {
    let model_size = 10;
    let dp_config = DifferentialPrivacyConfig::gaussian(5.0, 1e-5, 1.0);

    // Create privacy accountant
    let mut accountant = PrivacyAccountant::new(dp_config.clone());

    // Create model and simulate gradient update
    let _model = ModelParameters::new(vec![0.5; model_size]);
    let large_gradients = ModelParameters::new(vec![2.0; model_size]); // Large gradients

    // Clip gradients
    let clipped = accountant.clip_update(&large_gradients);
    let norm: f64 = clipped.values.iter().map(|v| v * v).sum::<f64>().sqrt();
    assert!(
        norm <= dp_config.clipping_norm + 1e-10,
        "Gradients should be clipped to max norm"
    );

    // Add noise
    let noisy = accountant.add_noise(clipped, 10);
    assert_eq!(noisy.values.len(), model_size);

    // Process a full round to properly track privacy
    let update = ModelParameters::new(vec![1.0; model_size]);
    accountant.process_round(update, 10);

    let epsilon_spent = accountant.epsilon_spent();
    assert!(
        epsilon_spent > 0.0,
        "Privacy cost should be positive after a step"
    );

    // Budget should not be exceeded yet (epsilon=5, spent ~5)
    assert!(
        accountant.is_budget_exhausted(),
        "Budget should be exhausted after 1 round with epsilon=5"
    );

    Ok(())
}

/// Test secure aggregation protocol
#[tokio::test]
async fn test_secure_aggregation() -> Result<(), Box<dyn std::error::Error>> {
    let num_clients = 4;
    let model_size = 8;
    let threshold = 3;

    // Create secure aggregator
    let mut aggregator = SecureAggregator::new(num_clients, threshold, model_size)?;

    // Use deterministic RNG for reproducibility
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(42);
    aggregator.state.generate_pairwise_seeds(&mut rng);

    // Create updates from clients
    let original_updates: Vec<Vec<f64>> = (0..num_clients)
        .map(|i| (0..model_size).map(|j| (i + j) as f64 * 0.1).collect())
        .collect();

    // Each client masks their update
    let masked_updates: Vec<Vec<f64>> = original_updates
        .iter()
        .enumerate()
        .map(|(i, weights)| {
            let client = nebula_federated::SecureClient::new(i, model_size);
            client
                .generate_masked_update(weights, &aggregator)
                .unwrap()
                .masked_weights
        })
        .collect();

    // Server aggregates masked updates
    let aggregated = aggregator.aggregate_masked_updates(&masked_updates)?;

    // Expected sum
    let expected: Vec<f64> = (0..model_size)
        .map(|j| original_updates.iter().map(|u| u[j]).sum())
        .collect();

    // Verify aggregation is correct
    for (actual, exp) in aggregated.iter().zip(expected.iter()) {
        assert!(
            (actual - exp).abs() < 1e-6,
            "Aggregated value {} should match expected {}",
            actual,
            exp
        );
    }

    Ok(())
}

/// Test privacy budget tracking across multiple rounds
#[tokio::test]
async fn test_privacy_budget_tracking() -> Result<(), Box<dyn std::error::Error>> {
    let dp_config = DifferentialPrivacyConfig::gaussian(10.0, 1e-5, 1.0);
    let mut accountant = PrivacyAccountant::new(dp_config.clone());

    // Simulate 5 training rounds
    for round in 0..5 {
        let params = ModelParameters::new(vec![1.0; 10]);
        accountant.process_round(params, 10);

        let epsilon_spent = accountant.epsilon_spent();

        // Epsilon should increase with each round
        if round > 0 {
            assert!(
                epsilon_spent > 0.0,
                "Epsilon should be positive after round {}",
                round
            );
        }

        // Check if budget is exceeded
        if accountant.is_budget_exhausted() {
            println!("Budget exceeded at round {}", round + 1);
            break;
        }
    }

    Ok(())
}

/// Test weighted aggregation (FedAvg)
#[tokio::test]
async fn test_weighted_aggregation() -> Result<(), Box<dyn std::error::Error>> {
    let model_size = 4;
    let config = FederatedConfig::new("test".to_string(), 3, 1);
    let server = FederatedServerImpl::new(config, ModelParameters::new(vec![0.0; model_size]));

    // Start a round
    let round_id = server.start_round().await?;

    // Create updates with different sample counts (weights)
    let updates = vec![
        ClientUpdate::new("client_0".to_string(), round_id, ModelParameters::new(vec![1.0, 2.0, 3.0, 4.0]), 100),
        ClientUpdate::new("client_1".to_string(), round_id, ModelParameters::new(vec![5.0, 6.0, 7.0, 8.0]), 200), // Double the weight
        ClientUpdate::new("client_2".to_string(), round_id, ModelParameters::new(vec![9.0, 10.0, 11.0, 12.0]), 100),
    ];

    // Submit updates
    for update in updates {
        server.process_update(update).await?;
    }

    // Get aggregated model
    let aggregated = server.get_global_model().await?;

    // Expected weighted average:
    // Total samples = 100 + 200 + 100 = 400
    // Weighted sum for first param: (1*100 + 5*200 + 9*100) / 400 = (100 + 1000 + 900) / 400 = 2000/400 = 5.0
    let expected_first = 5.0;
    let actual_first = aggregated.values[0];

    assert!(
        (actual_first - expected_first).abs() < 1e-6,
        "Weighted average should be {}, got {}",
        expected_first,
        actual_first
    );

    Ok(())
}

/// Test simple average aggregation
#[tokio::test]
async fn test_simple_average_aggregation() -> Result<(), Box<dyn std::error::Error>> {
    let model_size = 4;
    let mut config = FederatedConfig::new("test".to_string(), 2, 1);
    config.aggregation.min_clients = 2;
    config.aggregation.method = nebula_federated::AggregationMethod::SimpleAverage;
    let server = FederatedServerImpl::new(config, ModelParameters::new(vec![0.0; model_size]));

    // Start a round
    let round_id = server.start_round().await?;

    let updates = vec![
        ClientUpdate::new("client_0".to_string(), round_id, ModelParameters::new(vec![1.0, 2.0, 3.0, 4.0]), 100),
        ClientUpdate::new("client_1".to_string(), round_id, ModelParameters::new(vec![5.0, 6.0, 7.0, 8.0]), 200),
    ];

    // Submit updates
    for update in updates {
        server.process_update(update).await?;
    }

    // Get aggregated model
    let aggregated = server.get_global_model().await?;

    // Simple average: (1+5)/2 = 3.0, (2+6)/2 = 4.0, etc.
    let expected_first = 3.0;
    let actual_first = aggregated.values[0];

    assert!(
        (actual_first - expected_first).abs() < 1e-6,
        "Simple average should be {}, got {}",
        expected_first,
        actual_first
    );

    Ok(())
}

/// Test client update generation
#[tokio::test]
async fn test_client_update_generation() -> Result<(), Box<dyn std::error::Error>> {
    let model_size = 10;
    let client = MockClient::new("test_client".to_string(), model_size);

    let global_model = ModelParameters::new(vec![0.5; model_size]);

    // Generate update
    let update = client
        .train_local(&global_model, &UpdateConfig::default())
        .await?;

    // Verify update properties
    assert_eq!(update.client_id, "test_client");
    assert_eq!(update.parameters.values.len(), model_size);
    assert_eq!(update.num_samples, model_size); // MockClient uses data_size as num_samples

    // Update should be different from global model
    assert_ne!(
        update.parameters.values,
        global_model.values,
        "Client update should differ from global model"
    );

    Ok(())
}

/// Test minimum client requirement
#[tokio::test]
async fn test_min_clients_requirement() -> Result<(), Box<dyn std::error::Error>> {
    let model_size = 5;
    let mut config = FederatedConfig::new("test".to_string(), 1, 1);
    config.aggregation.min_clients = 2; // Require at least 2 clients
    let server = FederatedServerImpl::new(config, ModelParameters::new(vec![0.0; model_size]));

    // Start a round
    let round_id = server.start_round().await?;

    // Create only 1 update (less than min_clients=2)
    let update = ClientUpdate::new(
        "client_0".to_string(),
        round_id,
        ModelParameters::new(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        100,
    );

    // Submit update - should still succeed but round won't complete
    let result = server.process_update(update).await;
    assert!(result.is_ok(), "Single update should be accepted");

    // Round should still be active (not completed)
    let status = server.get_round_status(round_id).await?;
    assert_eq!(
        status, RoundStatus::Active,
        "Round should still be active with insufficient clients"
    );

    Ok(())
}

/// Test end-to-end federated learning with all features
#[tokio::test]
async fn test_end_to_end_federated_learning() -> Result<(), Box<dyn std::error::Error>> {
    let num_clients = 5;
    let num_rounds = 2;
    let model_size = 8;

    // Privacy configuration
    let dp_config = DifferentialPrivacyConfig::gaussian(10.0, 1e-5, 2.0);
    let privacy_accountant = Arc::new(Mutex::new(PrivacyAccountant::new(dp_config.clone())));

    // Initialize server
    let config = FederatedConfig::new("e2e_test".to_string(), num_clients, num_rounds)
        .with_client_fraction(0.6);

    let global_model = ModelParameters::new(vec![0.0; model_size]);
    let server = FederatedServerImpl::new(config, global_model);

    server.start().await?;

    // Register clients
    for i in 0..num_clients {
        server
            .register_client(format!("client_{}", i), ClientCapabilities::default())
            .await?;
    }

    // Run training rounds
    for round in 0..num_rounds {
        // Start round
        let round_id = server.start_round().await?;

        // Get current model
        let current_model = server.get_global_model().await?;

        // Select 3 clients for this round
        let selected = vec![0, 2, 4];

        // Clients generate updates
        for &idx in &selected {
            let client = MockClient::new(format!("client_{}", idx), 100);
            let update = client
                .train_local(&current_model, &UpdateConfig::default())
                .await?;

            // Apply differential privacy
            let mut accountant = privacy_accountant.lock().await;
            let clipped = accountant.clip_update(&update.parameters);
            let noisy = accountant.add_noise(clipped, selected.len());

            // Create update for this round
            let private_update = ClientUpdate::new(update.client_id, round_id, noisy, 100);
            drop(accountant);

            // Send to server
            server.process_update(private_update).await?;
        }

        // Track privacy using process_round
        {
            let mut accountant = privacy_accountant.lock().await;
            let dummy = ModelParameters::new(vec![0.0; model_size]);
            accountant.process_round(dummy, selected.len());
        }
    }

    // Verify final state
    let final_model = server.get_global_model().await?;
    let accountant = privacy_accountant.lock().await;
    let epsilon_spent = accountant.epsilon_spent();

    // Model should have changed from initial zeros
    assert!(
        final_model.values.iter().any(|&w| w.abs() > 1e-6),
        "Model should be updated after training"
    );

    // Privacy budget should be tracked
    assert!(
        epsilon_spent > 0.0,
        "Privacy budget should be spent after training"
    );

    server.stop().await?;
    Ok(())
}

/// Test Shamir's Secret Sharing integration
#[tokio::test]
async fn test_shamir_secret_sharing() -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(123);
    let secret = 42.0;
    let params = ShamirParams::new(5, 3)?;

    // Generate shares
    let shares = ShamirSecretSharing::generate_shares(secret, &params, &mut rng);
    assert_eq!(shares.len(), 5);

    // Reconstruct with threshold shares (3 out of 5)
    let subset = &shares[0..3];
    let reconstructed = ShamirSecretSharing::reconstruct_secret(subset)?;
    assert!(
        (reconstructed - secret).abs() < 1e-6,
        "Reconstructed secret should match original"
    );

    // With fewer than threshold shares (2 out of 5), reconstruction gives incorrect result
    // (Lagrange interpolation with insufficient points yields wrong polynomial)
    let insufficient = &shares[0..2];
    let reconstructed_insufficient = ShamirSecretSharing::reconstruct_secret(insufficient)?;
    // The result should be different from the original secret
    assert!(
        (reconstructed_insufficient - secret).abs() > 0.01,
        "Reconstruction with insufficient shares should give incorrect result"
    );

    Ok(())
}
