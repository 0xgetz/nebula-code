//! Federated Training Example
//!
//! This example demonstrates a complete federated learning workflow:
//! - Setting up a server and multiple clients
//! - Running federated training rounds with differential privacy
//! - Using secure aggregation for privacy-preserving model updates
//! - Tracking privacy budget across rounds
//!
//! Run with: cargo run --example federated_training

use nebula_federated::privacy::{DifferentialPrivacyConfig, PrivacyAccountant};
use nebula_federated::secure_aggregation::{SecureAggregator, SecureClient};
use nebula_federated::server::FederatedServerImpl;
use nebula_federated::types::{
    AggregationConfig, AggregationMethod, ClientUpdate, FederatedConfig, ModelConfig,
    ModelParameters, PrivacyConfig,
};
use nebula_federated::FederatedServer;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Federated Training Example ===\n");

    // Demonstrate differential privacy
    demonstrate_differential_privacy()?;

    // Demonstrate secure aggregation
    demonstrate_secure_aggregation()?;

    // Run a complete federated training simulation
    run_federated_training().await?;

    println!("\n=== Example completed successfully ===");
    Ok(())
}

/// Demonstrates differential privacy noise generation and budget tracking
fn demonstrate_differential_privacy() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Differential Privacy Demo ---\n");

    // Create a privacy configuration with Gaussian mechanism
    let config = DifferentialPrivacyConfig::gaussian(1.0, 1e-5, 1.0);

    println!("Privacy Configuration:");
    println!("  Epsilon: {}", config.epsilon);
    println!("  Delta: {}", config.delta);
    println!("  Clipping norm: {}", config.clipping_norm);
    println!("  Noise mechanism: {}", config.noise_mechanism);

    // Initialize privacy accountant
    let mut accountant = PrivacyAccountant::new(config);

    // Create sample model update (100 parameters)
    let update = ModelParameters::new(vec![0.5; 100]);

    println!("\nOriginal update (first 5 values): {:?}", &update.values[..5]);

    // Clip gradient by norm
    let clipped = accountant.clip_update(&update);
    println!("Clipped update (first 5 values): {:?}", &clipped.values[..5]);

    // Compute norm to verify clipping
    let norm: f64 = clipped.values.iter().map(|v| v * v).sum::<f64>().sqrt();
    println!("Clipped L2 norm: {:.4}", norm);

    // Add Gaussian noise
    let noisy = accountant.add_noise(clipped, 10); // 10 clients
    println!("Noisy update (first 5 values): {:?}", &noisy.values[..5]);

    // Process a round to track privacy budget
    let update2 = ModelParameters::new(vec![0.3; 100]);
    let _noisy2 = accountant.process_round(update2, 10);

    println!("\nPrivacy budget after 1 round:");
    println!("  Epsilon spent: {:.4}", accountant.epsilon_spent());
    println!("  Remaining budget: {:.4}", accountant.remaining_budget());
    println!("  Budget exhausted: {}", accountant.is_budget_exhausted());

    // Simulate multiple rounds
    println!("\nSimulating additional rounds:");
    for round in 2..=5 {
        let update = ModelParameters::new(vec![0.2; 100]);
        let _noisy = accountant.process_round(update, 10);
        println!(
            "  Round {}: epsilon spent = {:.4}, remaining = {:.4}",
            round,
            accountant.epsilon_spent(),
            accountant.remaining_budget()
        );
    }

    Ok(())
}

/// Demonstrates secure aggregation with pairwise masking
fn demonstrate_secure_aggregation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Secure Aggregation Demo ---\n");

    let num_clients = 5;
    let threshold = 3;
    let model_dim = 10;

    // Create secure aggregator
    let mut aggregator = SecureAggregator::new(num_clients, threshold, model_dim)?;

    // Initialize pairwise seeds
    let mut rng = ChaCha20Rng::seed_from_u64(42);
    aggregator.state.generate_pairwise_seeds(&mut rng);

    println!("Secure Aggregation Setup:");
    println!("  Clients: {}", aggregator.num_clients());
    println!("  Threshold: {}", aggregator.threshold());
    println!("  Model dimension: {}", aggregator.model_dim());

    // Simulate clients with model updates
    let original_updates: Vec<Vec<f64>> = (0..num_clients)
        .map(|i| (0..model_dim).map(|j| (i + j) as f64 * 0.1).collect())
        .collect();

    println!("\nOriginal updates (first 3 values per client):");
    for (i, update) in original_updates.iter().enumerate() {
        println!("  Client {}: {:?}", i, &update[..3]);
    }

    // Each client masks their update
    let masked_updates: Vec<Vec<f64>> = original_updates
        .iter()
        .enumerate()
        .map(|(i, weights)| {
            let client = SecureClient::new(i, model_dim);
            client
                .generate_masked_update(weights, &aggregator)
                .unwrap()
                .masked_weights
        })
        .collect();

    println!("\nMasked updates (first 3 values per client):");
    for (i, masked) in masked_updates.iter().enumerate() {
        println!("  Client {}: {:?}", i, &masked[..3]);
    }

    // Server aggregates masked updates
    let aggregated = aggregator.aggregate_masked_updates(&masked_updates)?;

    // Expected sum
    let expected: Vec<f64> = (0..model_dim)
        .map(|j| original_updates.iter().map(|u| u[j]).sum())
        .collect();

    println!("\nAggregation results (first 3 values):");
    println!("  Aggregated: {:?}", &aggregated[..3]);
    println!("  Expected:   {:?}", &expected[..3]);

    // Verify correctness
    let tolerance = 1e-6;
    let matches = aggregated
        .iter()
        .zip(expected.iter())
        .all(|(a, b)| (a - b).abs() < tolerance);
    println!("\nSecure aggregation correct: {}", matches);

    Ok(())
}

/// Runs a complete federated training simulation
async fn run_federated_training() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n--- Federated Training Simulation ---\n");

    // Configuration
    let num_clients = 5;
    let num_rounds = 3;
    let model_size = 50;
    let min_clients = 3;

    // Create federated configuration with privacy enabled
    let fed_config = FederatedConfig::new("demo_job".to_string(), num_clients, num_rounds)
        .with_client_fraction(0.8)
        .with_privacy(PrivacyConfig {
            enabled: true,
            epsilon: 5.0, // Higher budget for multiple rounds
            delta: 1e-5,
            noise_multiplier: 1.1,
            clipping_norm: 1.0,
        });

    // Customize aggregation config
    let fed_config = FederatedConfig {
        aggregation: AggregationConfig {
            min_clients,
            max_clients: num_clients,
            timeout_ms: 60000,
            method: AggregationMethod::FedAvg,
        },
        model: ModelConfig {
            architecture: "mlp".to_string(),
            num_params: model_size,
            learning_rate: 0.01,
            batch_size: 32,
            local_epochs: 5,
        },
        ..fed_config
    };

    println!("Federated Configuration:");
    println!("  Job ID: {}", fed_config.job_id);
    println!("  Total clients: {}", fed_config.total_clients);
    println!("  Rounds: {}", fed_config.num_rounds);
    println!("  Client fraction: {:.1}%", fed_config.client_fraction * 100.0);
    println!("  Min clients per round: {}", fed_config.aggregation.min_clients);
    println!("  Privacy enabled: {}", fed_config.privacy.enabled);
    println!("  Privacy epsilon: {}", fed_config.privacy.epsilon);
    println!("  Model architecture: {}", fed_config.model.architecture);
    println!("  Model parameters: {}", fed_config.model.num_params);

    // Initialize global model
    let initial_model = ModelParameters::new(vec![0.0; model_size]);

    // Create server
    let server = FederatedServerImpl::new(fed_config.clone(), initial_model);

    // Start server
    server.start().await?;

    // Register mock clients
    for i in 0..num_clients {
        let client_id = format!("client_{}", i);
        server
            .register_client(client_id.clone(), Default::default())
            .await?;
        println!("  Registered {}", client_id);
    }

    // Create privacy accountant for tracking
    let dp_config = DifferentialPrivacyConfig::gaussian(
        fed_config.privacy.epsilon,
        fed_config.privacy.delta,
        fed_config.privacy.clipping_norm,
    );
    let mut privacy_accountant = PrivacyAccountant::new(dp_config);

    println!("\nStarting federated training...\n");

    // Run training rounds
    for round in 0..num_rounds {
        println!("=== Round {} ===", round + 1);

        // Start round
        let round_id = server.start_round().await?;
        println!("  Round {} started (min_clients={})", round_id, min_clients);

        // Simulate client updates - send exactly min_clients updates to complete the round
        let num_participating = min_clients;
        for i in 0..num_participating {
            let client_id = format!("client_{}", i);

            // Create mock client update
            let update_values: Vec<f64> = (0..model_size)
                .map(|j| (round + i + j) as f64 * 0.01)
                .collect();
            let params = ModelParameters::new(update_values);

            // Apply differential privacy if enabled
            let dp_params = if fed_config.privacy.enabled {
                let clipped = privacy_accountant.clip_update(&params);
                privacy_accountant.add_noise(clipped, num_participating)
            } else {
                params
            };

            // Create client update
            let update = ClientUpdate::new(
                client_id,
                round_id,
                dp_params,
                100, // num_samples
            );

            // Submit update to server
            let completed = server.process_update(update).await?;
            println!(
                "  Client {} submitted update (round completed: {})",
                i, completed
            );
        }

        // Check round status
        let status = server.get_round_status(round_id).await?;
        println!("  Round status: {:?}", status);

        // Get updated global model
        let global_model = server.get_global_model().await?;
        let model_norm: f64 = global_model
            .values
            .iter()
            .map(|v| v * v)
            .sum::<f64>()
            .sqrt();
        println!("  Global model L2 norm: {:.4}", model_norm);

        // Print privacy budget
        if fed_config.privacy.enabled {
            println!(
                "  Privacy budget: {:.4} / {:.4}",
                privacy_accountant.epsilon_spent(),
                privacy_accountant.config().epsilon
            );
        }

        println!();
    }

    // Final results
    println!("Training completed!");
    println!(
        "Final privacy budget spent: {:.4}",
        privacy_accountant.epsilon_spent()
    );

    // Get final global model
    let final_model = server.get_global_model().await?;
    println!(
        "Final model L2 norm: {:.4}",
        final_model.values.iter().map(|v| v * v).sum::<f64>().sqrt()
    );

    Ok(())
}
