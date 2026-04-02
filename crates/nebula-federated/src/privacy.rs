//! # Differential Privacy Module
//!
//! This module provides differential privacy mechanisms for federated learning,
//! including Laplace and Gaussian noise generation, privacy budget tracking,
//! gradient clipping, and private aggregation.
//!
//! ## Overview
//!
//! Differential privacy ensures that the contribution of any single client
//! cannot be distinguished in the aggregated model updates. This is achieved
//! through:
//!
//! 1. **Gradient Clipping**: Bound the influence of each client by clipping
//!    their update to a maximum L2 norm.
//! 2. **Noise Addition**: Add calibrated noise (Laplace or Gaussian) to the
//!    aggregated updates.
//! 3. **Privacy Accounting**: Track the cumulative privacy loss (epsilon, delta)
//!    across multiple rounds.
//!
//! ## Example
//!
//! ```rust
//! use nebula_federated::privacy::{
//!     PrivacyAccountant, NoiseMechanism, DifferentialPrivacyConfig,
//! };
//! use nebula_federated::types::ModelParameters;
//!
//! // Configure differential privacy
//! let config = DifferentialPrivacyConfig::new(
//!     1.0,   // epsilon
//!     1e-5,  // delta
//!     1.0,   // clipping_norm
//!     NoiseMechanism::Gaussian { sigma: 0.1 },
//! );
//!
//! // Create accountant
//! let mut accountant = PrivacyAccountant::new(config);
//!
//! // Process client updates with privacy
//! let update = ModelParameters::new(vec![1.0; 100]);
//! let clipped = accountant.clip_update(&update);
//! let noisy = accountant.add_noise(clipped, 10); // 10 clients
//! ```

use rand::distributions::Distribution;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rand_distr::Normal;

use crate::types::ModelParameters;

/// Privacy mechanism for adding noise to model updates.
#[derive(Debug, Clone, PartialEq)]
pub enum NoiseMechanism {
    /// Laplace mechanism: adds noise from Laplace distribution.
    /// Provides pure epsilon-differential privacy (delta = 0).
    Laplace {
        /// Scale parameter b = sensitivity / epsilon
        scale: f64,
    },
    /// Gaussian mechanism: adds noise from Gaussian distribution.
    /// Provides (epsilon, delta)-differential privacy.
    Gaussian {
        /// Standard deviation of the Gaussian noise
        sigma: f64,
    },
}

impl std::fmt::Display for NoiseMechanism {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoiseMechanism::Laplace { scale } => write!(f, "Laplace(scale={:.4})", scale),
            NoiseMechanism::Gaussian { sigma } => write!(f, "Gaussian(sigma={:.4})", sigma),
        }
    }
}

/// Configuration for differential privacy in federated learning.
#[derive(Debug, Clone)]
pub struct DifferentialPrivacyConfig {
    /// Privacy budget epsilon (must be > 0). Smaller values provide stronger privacy.
    pub epsilon: f64,
    /// Privacy parameter delta (probability of privacy failure). Must be in [0, 1).
    pub delta: f64,
    /// Maximum L2 norm for gradient clipping.
    pub clipping_norm: f64,
    /// Noise mechanism to use.
    pub noise_mechanism: NoiseMechanism,
}

impl DifferentialPrivacyConfig {
    /// Create a new differential privacy configuration.
    ///
    /// # Arguments
    /// * `epsilon` - Privacy budget (must be > 0)
    /// * `delta` - Privacy failure probability (must be in [0, 1))
    /// * `clipping_norm` - Maximum L2 norm for gradients
    /// * `mechanism` - Noise mechanism to use
    ///
    /// # Panics
    /// Panics if epsilon <= 0 or delta is not in [0, 1).
    pub fn new(
        epsilon: f64,
        delta: f64,
        clipping_norm: f64,
        mechanism: NoiseMechanism,
    ) -> Self {
        assert!(epsilon > 0.0, "epsilon must be positive");
        assert!(delta >= 0.0 && delta < 1.0, "delta must be in [0, 1)");
        assert!(clipping_norm > 0.0, "clipping_norm must be positive");

        Self {
            epsilon,
            delta,
            clipping_norm,
            noise_mechanism: mechanism,
        }
    }

    /// Create a configuration with Laplace mechanism.
    pub fn laplace(epsilon: f64, clipping_norm: f64) -> Self {
        let scale = clipping_norm / epsilon;
        Self::new(
            epsilon,
            0.0, // Laplace provides pure DP
            clipping_norm,
            NoiseMechanism::Laplace { scale },
        )
    }

    /// Create a configuration with Gaussian mechanism.
    ///
    /// The sigma is computed automatically to satisfy (epsilon, delta)-DP
    /// using the standard Gaussian mechanism bound.
    pub fn gaussian(epsilon: f64, delta: f64, clipping_norm: f64) -> Self {
        // Using the standard Gaussian mechanism: sigma = C * sqrt(2 * ln(1.25/delta)) / epsilon
        let c = clipping_norm;
        let delta_safe = delta.max(1e-30); // Avoid division by zero
        let sigma = c * (2.0 * (1.25 / delta_safe).ln()).sqrt() / epsilon;

        // Ensure sigma is positive and reasonable
        let sigma = sigma.max(1e-10);

        Self::new(
            epsilon,
            delta,
            clipping_norm,
            NoiseMechanism::Gaussian { sigma },
        )
    }
}

/// Tracks cumulative privacy loss across multiple rounds of federated learning.
#[derive(Debug)]
pub struct PrivacyAccountant {
    /// Configuration for differential privacy.
    config: DifferentialPrivacyConfig,
    /// Total epsilon spent so far.
    epsilon_spent: f64,
    /// Number of rounds processed.
    rounds_completed: u32,
    /// Random number generator for reproducibility.
    rng: StdRng,
}

impl PrivacyAccountant {
    /// Create a new privacy accountant with the given configuration.
    pub fn new(config: DifferentialPrivacyConfig) -> Self {
        Self::with_seed(config, 42)
    }

    /// Create a new privacy accountant with a specific random seed.
    pub fn with_seed(config: DifferentialPrivacyConfig, seed: u64) -> Self {
        Self {
            config,
            epsilon_spent: 0.0,
            rounds_completed: 0,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Get the current privacy configuration.
    pub fn config(&self) -> &DifferentialPrivacyConfig {
        &self.config
    }

    /// Get the total epsilon spent so far.
    pub fn epsilon_spent(&self) -> f64 {
        self.epsilon_spent
    }

    /// Get the number of rounds completed.
    pub fn rounds_completed(&self) -> u32 {
        self.rounds_completed
    }

    /// Check if the privacy budget has been exhausted.
    pub fn is_budget_exhausted(&self) -> bool {
        self.epsilon_spent >= self.config.epsilon
    }

    /// Get the remaining privacy budget (epsilon).
    pub fn remaining_budget(&self) -> f64 {
        (self.config.epsilon - self.epsilon_spent).max(0.0)
    }

    /// Clip a model update to have L2 norm at most `clipping_norm`.
    ///
    /// This bounds the influence of any single client's update.
    pub fn clip_update(&self, update: &ModelParameters) -> ModelParameters {
        clip_gradient_by_norm(update, self.config.clipping_norm)
    }

    /// Add noise to a clipped model update.
    ///
    /// # Arguments
    /// * `update` - The clipped model update
    /// * `num_clients` - Number of clients participating in this round
    ///
    /// # Returns
    /// The noisy model update.
    pub fn add_noise(&mut self, update: ModelParameters, num_clients: usize) -> ModelParameters {
        match &self.config.noise_mechanism {
            NoiseMechanism::Laplace { scale } => {
                // Scale noise by 1/num_clients for aggregation
                let noise_scale = scale / num_clients as f64;
                add_laplace_noise(update, noise_scale, &mut self.rng)
            }
            NoiseMechanism::Gaussian { sigma } => {
                // Scale noise by 1/num_clients for aggregation
                let noise_sigma = sigma / num_clients as f64;
                add_gaussian_noise(update, noise_sigma, &mut self.rng)
            }
        }
    }

    /// Process a round of federated learning with privacy.
    ///
    /// This clips the aggregated update, adds noise, and tracks privacy loss.
    ///
    /// # Arguments
    /// * `aggregated_update` - The aggregated (but not yet noised) model update
    /// * `num_clients` - Number of clients participating
    ///
    /// # Returns
    /// The differentially private aggregated update.
    pub fn process_round(
        &mut self,
        aggregated_update: ModelParameters,
        num_clients: usize,
    ) -> ModelParameters {
        // Clip the aggregated update (should already be bounded if clients clipped individually)
        let clipped = self.clip_update(&aggregated_update);

        // Add noise
        let noisy = self.add_noise(clipped, num_clients);

        // Update privacy accounting
        self.rounds_completed += 1;

        // Update epsilon spent based on the mechanism
        // For simplicity, we use basic composition: each round spends the full epsilon
        self.epsilon_spent += self.config.epsilon;

        noisy
    }

    /// Compute the privacy loss for a given number of rounds using advanced composition.
    ///
    /// This uses the advanced composition theorem for (epsilon, delta)-DP.
    pub fn compute_privacy_loss(&self, num_rounds: u32) -> (f64, f64) {
        match &self.config.noise_mechanism {
            NoiseMechanism::Laplace { .. } => {
                // Basic composition: k * epsilon
                let epsilon_total = self.config.epsilon * num_rounds as f64;
                (epsilon_total, 0.0)
            }
            NoiseMechanism::Gaussian { sigma } => {
                let c = self.config.clipping_norm;
                let delta_val = self.config.delta.max(1e-30);
                let epsilon_0 = (2.0 * (1.25 / delta_val).ln()).sqrt() * c / sigma;

                // Advanced composition
                let k = num_rounds as f64;
                let delta_prime = delta_val;
                let sqrt_term = (2.0 * k * (1.0 / delta_prime).ln()).sqrt().max(0.0);
                let exp_term = (epsilon_0.exp() - 1.0).max(0.0);
                let epsilon_total = epsilon_0 * sqrt_term + k * epsilon_0 * exp_term;

                (epsilon_total, self.config.delta)
            }
        }
    }
}

/// Clip a gradient vector by its L2 norm.
///
/// If the L2 norm exceeds `max_norm`, the vector is scaled down to have
/// exactly `max_norm` as its L2 norm. Otherwise, it is returned unchanged.
pub fn clip_gradient_by_norm(params: &ModelParameters, max_norm: f64) -> ModelParameters {
    let values = &params.values;
    let norm = l2_norm(values);

    if norm <= max_norm {
        params.clone()
    } else {
        let scale = max_norm / norm;
        ModelParameters {
            values: values.iter().map(|&v| v * scale).collect(),
            metadata: params.metadata.clone(),
        }
    }
}

/// Compute the L2 norm of a vector.
fn l2_norm(values: &[f64]) -> f64 {
    values.iter().map(|&v| v * v).sum::<f64>().sqrt()
}

/// Add Laplace noise to each element of the model parameters.
///
/// Uses the inverse transform method: X = -b * sign(U) * ln(1 - 2|U|)
/// where U ~ Uniform(-0.5, 0.5).
fn add_laplace_noise(params: ModelParameters, scale: f64, rng: &mut StdRng) -> ModelParameters {
    if scale <= 0.0 {
        return params;
    }

    let noisy_values: Vec<f64> = params
        .values
        .iter()
        .map(|&v| {
            let u: f64 = rng.gen_range(-0.5..0.5);
            let laplace_sample = scale * u.signum() * (1.0 - 2.0 * u.abs()).ln().abs();
            v + laplace_sample
        })
        .collect();

    ModelParameters {
        values: noisy_values,
        metadata: params.metadata,
    }
}

/// Add Gaussian noise to each element of the model parameters.
fn add_gaussian_noise(params: ModelParameters, sigma: f64, rng: &mut StdRng) -> ModelParameters {
    if sigma <= 0.0 {
        return params;
    }

    let normal = Normal::new(0.0, sigma).expect("Invalid Normal distribution parameters");
    let noisy_values: Vec<f64> = params
        .values
        .iter()
        .map(|&v| v + normal.sample(rng))
        .collect();

    ModelParameters {
        values: noisy_values,
        metadata: params.metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_params(values: Vec<f64>) -> ModelParameters {
        ModelParameters::new(values)
    }

    #[test]
    fn test_l2_norm() {
        let values = vec![3.0, 4.0];
        assert!((l2_norm(&values) - 5.0).abs() < 1e-10);

        let values = vec![1.0, 0.0, 0.0];
        assert!((l2_norm(&values) - 1.0).abs() < 1e-10);

        let values = vec![0.0; 10];
        assert!((l2_norm(&values) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_clip_gradient_by_norm_no_change() {
        let params = make_params(vec![3.0, 4.0]);
        let clipped = clip_gradient_by_norm(&params, 10.0);
        assert_eq!(clipped.values, vec![3.0, 4.0]);
    }

    #[test]
    fn test_clip_gradient_by_norm_clips() {
        let params = make_params(vec![3.0, 4.0]);
        let clipped = clip_gradient_by_norm(&params, 2.5);
        assert!((clipped.values[0] - 1.5).abs() < 1e-10);
        assert!((clipped.values[1] - 2.0).abs() < 1e-10);
        let norm = l2_norm(&clipped.values);
        assert!((norm - 2.5).abs() < 1e-10);
    }

    #[test]
    fn test_clip_gradient_by_norm_zero_vector() {
        let params = make_params(vec![0.0; 10]);
        let clipped = clip_gradient_by_norm(&params, 1.0);
        assert_eq!(clipped.values, vec![0.0; 10]);
    }

    #[test]
    fn test_dp_config_validation() {
        // Should panic on invalid epsilon
        let result = std::panic::catch_unwind(|| {
            DifferentialPrivacyConfig::new(0.0, 1e-5, 1.0, NoiseMechanism::Gaussian { sigma: 0.1 })
        });
        assert!(result.is_err());

        // Should panic on invalid delta (negative)
        let result = std::panic::catch_unwind(|| {
            DifferentialPrivacyConfig::new(1.0, -0.1, 1.0, NoiseMechanism::Gaussian { sigma: 0.1 })
        });
        assert!(result.is_err());

        // Should panic on invalid delta (>= 1)
        let result = std::panic::catch_unwind(|| {
            DifferentialPrivacyConfig::new(1.0, 1.0, 1.0, NoiseMechanism::Gaussian { sigma: 0.1 })
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_dp_config_laplace() {
        let config = DifferentialPrivacyConfig::laplace(1.0, 2.0);
        assert_eq!(config.epsilon, 1.0);
        assert_eq!(config.delta, 0.0);
        assert_eq!(config.clipping_norm, 2.0);
        match config.noise_mechanism {
            NoiseMechanism::Laplace { scale } => assert!((scale - 2.0).abs() < 1e-10),
            _ => panic!("Expected Laplace mechanism"),
        }
    }

    #[test]
    fn test_dp_config_gaussian() {
        let config = DifferentialPrivacyConfig::gaussian(1.0, 1e-5, 1.0);
        assert_eq!(config.epsilon, 1.0);
        assert_eq!(config.delta, 1e-5);
        assert_eq!(config.clipping_norm, 1.0);
        match config.noise_mechanism {
            NoiseMechanism::Gaussian { sigma } => assert!(sigma > 0.0),
            _ => panic!("Expected Gaussian mechanism"),
        }
    }

    #[test]
    fn test_privacy_accountant_creation() {
        let config = DifferentialPrivacyConfig::laplace(1.0, 1.0);
        let accountant = PrivacyAccountant::new(config);

        assert_eq!(accountant.epsilon_spent(), 0.0);
        assert_eq!(accountant.rounds_completed(), 0);
        assert!(!accountant.is_budget_exhausted());
        assert!((accountant.remaining_budget() - 1.0).abs() < 1e-10);
    }

    #[test]
    #[test]
    fn test_privacy_accountant_laplace_round() {
        let config = DifferentialPrivacyConfig::laplace(1.0, 1.0);
        let mut accountant = PrivacyAccountant::with_seed(config, 123);

        let params = make_params(vec![1.0; 100]);
        let noisy = accountant.process_round(params, 10);

        assert_eq!(accountant.rounds_completed(), 1);
        assert!((accountant.epsilon_spent() - 1.0).abs() < 1e-10);
        assert!(accountant.is_budget_exhausted());

        // With Laplace noise scale = 1/10 = 0.1, values should be around 1.0
        // Allow generous bounds due to randomness
        let noisy_mean: f64 = noisy.values.iter().sum::<f64>() / noisy.values.len() as f64;
        assert!((noisy_mean - 1.0).abs() < 1.0, "Mean {} should be within 1.0 of 1.0", noisy_mean);
    }

    #[test]
    fn test_privacy_accountant_gaussian_round() {
        let config = DifferentialPrivacyConfig::gaussian(1.0, 1e-5, 1.0);
        let mut accountant = PrivacyAccountant::new(config);

        let params = make_params(vec![2.0; 50]);
        let noisy = accountant.process_round(params, 5);

        assert_eq!(accountant.rounds_completed(), 1);
        assert!((accountant.epsilon_spent() - 1.0).abs() < 1e-10);
        assert!(accountant.is_budget_exhausted());

        // Verify noise is added but values are still around 2.0
        let noisy_mean: f64 = noisy.values.iter().sum::<f64>() / noisy.values.len() as f64;
        assert!((noisy_mean - 2.0).abs() < 2.0);
    }

    #[test]
    fn test_privacy_accountant_multiple_rounds() {
        let config = DifferentialPrivacyConfig::laplace(2.0, 1.0);
        let mut accountant = PrivacyAccountant::new(config);

        for _ in 0..3 {
            let params = make_params(vec![1.0; 10]);
            accountant.process_round(params, 10);
        }

        assert_eq!(accountant.rounds_completed(), 3);
        assert!((accountant.epsilon_spent() - 6.0).abs() < 1e-10);
        assert!(accountant.is_budget_exhausted());
    }

    #[test]
    fn test_privacy_accountant_remaining_budget() {
        let config = DifferentialPrivacyConfig::laplace(5.0, 1.0);
        let mut accountant = PrivacyAccountant::new(config);

        assert!((accountant.remaining_budget() - 5.0).abs() < 1e-10);

        // Spend epsilon per round
        let params = make_params(vec![1.0; 10]);
        accountant.process_round(params, 10);
        assert!((accountant.remaining_budget() - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_add_laplace_noise_deterministic_with_seed() {
        let config = DifferentialPrivacyConfig::laplace(1.0, 1.0);
        let mut accountant1 = PrivacyAccountant::with_seed(config.clone(), 12345);
        let mut accountant2 = PrivacyAccountant::with_seed(config, 12345);

        let params = make_params(vec![1.0; 20]);
        let noisy1 = accountant1.add_noise(params.clone(), 1);
        let noisy2 = accountant2.add_noise(params, 1);

        assert_eq!(noisy1.values, noisy2.values);
    }

    #[test]
    fn test_add_gaussian_noise_deterministic_with_seed() {
        let config = DifferentialPrivacyConfig::gaussian(1.0, 1e-5, 1.0);
        let mut accountant1 = PrivacyAccountant::with_seed(config.clone(), 54321);
        let mut accountant2 = PrivacyAccountant::with_seed(config, 54321);

        let params = make_params(vec![2.0; 30]);
        let noisy1 = accountant1.add_noise(params.clone(), 1);
        let noisy2 = accountant2.add_noise(params, 1);

        assert_eq!(noisy1.values, noisy2.values);
    }

    #[test]
    fn test_noise_scale_with_num_clients() {
        let config = DifferentialPrivacyConfig::laplace(1.0, 1.0);
        let mut accountant = PrivacyAccountant::with_seed(config, 999);

        let params = make_params(vec![1.0; 100]);

        let noisy_10 = accountant.add_noise(params.clone(), 10);
        let noisy_100 = accountant.add_noise(params, 100);

        assert_eq!(noisy_10.values.len(), 100);
        assert_eq!(noisy_100.values.len(), 100);
    }

    #[test]
    fn test_compute_privacy_loss_laplace() {
        let config = DifferentialPrivacyConfig::laplace(1.0, 1.0);
        let accountant = PrivacyAccountant::new(config);

        let (eps, delta) = accountant.compute_privacy_loss(10);
        assert!((eps - 10.0).abs() < 1e-10);
        assert!((delta - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_compute_privacy_loss_gaussian() {
        let config = DifferentialPrivacyConfig::gaussian(1.0, 1e-5, 1.0);
        let accountant = PrivacyAccountant::new(config);

        let (eps, delta) = accountant.compute_privacy_loss(10);
        assert!(eps > 0.0);
        assert!((delta - 1e-5).abs() < 1e-10);
    }

    #[test]
    #[test]
    fn test_clip_then_noise_preserves_approximate_mean() {
        let config = DifferentialPrivacyConfig::gaussian(0.5, 1e-5, 100.0);
        let mut accountant = PrivacyAccountant::with_seed(config, 42);

        let original_value = 0.01;
        let params = make_params(vec![original_value; 1000]);

        let clipped = accountant.clip_update(&params);
        let noisy = accountant.add_noise(clipped, 100);

        let mean: f64 = noisy.values.iter().sum::<f64>() / noisy.values.len() as f64;
        // With Gaussian noise and 100 clients, the noise per element is very small
        // Allow reasonable bounds
        assert!(
            (mean - original_value).abs() < 0.1,
            "Mean {} should be close to {}",
            mean,
            original_value
        );
    }

    #[test]
    fn test_noise_mechanism_display() {
        let laplace = NoiseMechanism::Laplace { scale: 0.5 };
        assert_eq!(format!("{}", laplace), "Laplace(scale=0.5000)");

        let gaussian = NoiseMechanism::Gaussian { sigma: 1.2345 };
        assert_eq!(format!("{}", gaussian), "Gaussian(sigma=1.2345)");
    }

    #[test]
    fn test_metadata_preserved() {
        let config = DifferentialPrivacyConfig::laplace(1.0, 1.0);
        let mut accountant = PrivacyAccountant::new(config);

        let mut params = make_params(vec![1.0; 10]);
        params.metadata.model_name = Some("test_model".to_string());

        let clipped = accountant.clip_update(&params);
        assert_eq!(clipped.metadata.model_name, Some("test_model".to_string()));

        let noisy = accountant.add_noise(clipped, 10);
        assert_eq!(noisy.metadata.model_name, Some("test_model".to_string()));
    }
}
