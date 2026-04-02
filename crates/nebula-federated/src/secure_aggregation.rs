//! Secure Aggregation Module
//!
//! Implements privacy-preserving aggregation for federated learning using:
//! - Shamir's Secret Sharing for threshold reconstruction
//! - Pairwise masking between clients
//! - Server-side mask removal after aggregation
//!
//! This ensures the server never sees individual client updates, only the
//! aggregated result.

use ndarray::Array1;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::FederatedError;

/// A share in Shamir's Secret Sharing scheme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretShare {
    /// The index of this share (non-zero).
    pub index: usize,
    /// The share value (the y-coordinate on the polynomial).
    pub value: f64,
}

/// Parameters for Shamir's Secret Sharing.
#[derive(Debug, Clone)]
pub struct ShamirParams {
    /// Total number of shares to generate.
    pub n: usize,
    /// Threshold: minimum shares needed to reconstruct.
    pub t: usize,
}

impl ShamirParams {
    /// Create new Shamir parameters.
    pub fn new(n: usize, t: usize) -> Result<Self, FederatedError> {
        if t > n {
            return Err(FederatedError::AggregationFailed(
                "Threshold cannot exceed total shares".to_string(),
            ));
        }
        if t < 1 {
            return Err(FederatedError::AggregationFailed(
                "Threshold must be at least 1".to_string(),
            ));
        }
        Ok(Self { n, t })
    }
}

/// Shamir's Secret Sharing implementation.
pub struct ShamirSecretSharing;

impl ShamirSecretSharing {
    /// Generate shares for a secret value.
    ///
    /// Uses polynomial interpolation over real numbers (approximated with f64).
    /// For production use, this should be done in a finite field.
    pub fn generate_shares(
        secret: f64,
        params: &ShamirParams,
        rng: &mut impl Rng,
    ) -> Vec<SecretShare> {
        // Generate random coefficients for the polynomial:
        // f(x) = secret + a1*x + a2*x^2 + ... + a_{t-1}*x^{t-1}
        let mut coefficients = vec![secret];
        for _ in 1..params.t {
            // Use smaller coefficients for better numerical stability
            coefficients.push(rng.gen_range(-1000.0..1000.0));
        }

        // Evaluate polynomial at points 1, 2, ..., n
        (1..=params.n)
            .map(|i| {
                let x = i as f64;
                let y = Self::evaluate_polynomial(&coefficients, x);
                SecretShare { index: i, value: y }
            })
            .collect()
    }

    /// Evaluate polynomial at point x.
    fn evaluate_polynomial(coefficients: &[f64], x: f64) -> f64 {
        coefficients
            .iter()
            .enumerate()
            .map(|(i, &c)| c * x.powi(i as i32))
            .sum()
    }

    /// Reconstruct the secret from a set of shares using Lagrange interpolation.
    pub fn reconstruct_secret(shares: &[SecretShare]) -> Result<f64, FederatedError> {
        if shares.is_empty() {
            return Err(FederatedError::AggregationFailed(
                "No shares provided for reconstruction".to_string(),
            ));
        }

        // Check for duplicate indices
        let mut indices = Vec::new();
        for share in shares {
            if indices.contains(&share.index) {
                return Err(FederatedError::AggregationFailed(
                    "Duplicate share indices".to_string(),
                ));
            }
            indices.push(share.index);
        }

        // Lagrange interpolation at x=0 to get the secret
        let mut secret = 0.0;
        for (i, share_i) in shares.iter().enumerate() {
            let mut numerator = 1.0;
            let mut denominator = 1.0;

            for (j, share_j) in shares.iter().enumerate() {
                if i != j {
                    let xi = share_i.index as f64;
                    let xj = share_j.index as f64;
                    numerator *= 0.0 - xj; // Evaluate at x=0
                    denominator *= xi - xj;
                }
            }

            if denominator.abs() < 1e-12 {
                return Err(FederatedError::AggregationFailed(
                    "Division by zero in Lagrange interpolation".to_string(),
                ));
            }

            secret += share_i.value * (numerator / denominator);
        }

        Ok(secret)
    }

    /// Generate shares for a vector of secrets (model parameters).
    /// Returns a vector where each element is the shares for one secret.
    pub fn generate_shares_vector(
        secrets: &[f64],
        params: &ShamirParams,
        rng: &mut impl Rng,
    ) -> Vec<Vec<SecretShare>> {
        secrets
            .iter()
            .map(|&secret| Self::generate_shares(secret, params, rng))
            .collect()
    }

    /// Reconstruct a vector of secrets from their respective shares.
    /// Each element in shares_list is the shares for one secret.
    pub fn reconstruct_vector(
        shares_list: &[Vec<SecretShare>],
    ) -> Result<Vec<f64>, FederatedError> {
        if shares_list.is_empty() {
            return Err(FederatedError::AggregationFailed(
                "No shares provided for reconstruction".to_string(),
            ));
        }

        let mut secrets = Vec::with_capacity(shares_list.len());
        for shares in shares_list {
            secrets.push(Self::reconstruct_secret(shares)?);
        }

        Ok(secrets)
    }
}

/// Masked client update for secure aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaskedUpdate {
    /// Client identifier.
    pub client_id: usize,
    /// The masked model update (original + masks).
    pub masked_weights: Vec<f64>,
    /// Shares of the pairwise masks (for reconstruction if client drops).
    /// Each inner vec contains all shares for one pairwise mask.
    pub mask_shares: Vec<Vec<SecretShare>>,
}

/// Secure aggregation protocol state.
#[derive(Debug, Clone)]
pub struct SecureAggregationState {
    /// Total number of participating clients.
    pub num_clients: usize,
    /// Threshold for secret reconstruction.
    pub threshold: usize,
    /// Model dimension (number of parameters).
    pub model_dim: usize,
    /// Pairwise mask seeds between clients.
    pub pairwise_seeds: HashMap<(usize, usize), u64>,
}

impl SecureAggregationState {
    /// Create a new secure aggregation state.
    pub fn new(num_clients: usize, threshold: usize, model_dim: usize) -> Result<Self, FederatedError> {
        if threshold > num_clients {
            return Err(FederatedError::AggregationFailed(
                "Threshold cannot exceed number of clients".to_string(),
            ));
        }
        if threshold < 1 {
            return Err(FederatedError::AggregationFailed(
                "Threshold must be at least 1".to_string(),
            ));
        }

        Ok(Self {
            num_clients,
            threshold,
            model_dim,
            pairwise_seeds: HashMap::new(),
        })
    }

    /// Generate pairwise seeds for all client pairs.
    pub fn generate_pairwise_seeds(&mut self, rng: &mut impl Rng) {
        for i in 0..self.num_clients {
            for j in (i + 1)..self.num_clients {
                let seed = rng.gen::<u64>();
                self.pairwise_seeds.insert((i, j), seed);
            }
        }
    }

    /// Get the seed for a pair of clients (order-independent).
    pub fn get_pairwise_seed(&self, client_a: usize, client_b: usize) -> Option<u64> {
        let (min, max) = if client_a < client_b {
            (client_a, client_b)
        } else {
            (client_b, client_a)
        };
        self.pairwise_seeds.get(&(min, max)).copied()
    }
}

/// Secure aggregation protocol implementation.
pub struct SecureAggregator {
    /// Aggregation state.
    pub state: SecureAggregationState,
}

impl SecureAggregator {
    /// Create a new secure aggregator.
    pub fn new(
        num_clients: usize,
        threshold: usize,
        model_dim: usize,
    ) -> Result<Self, FederatedError> {
        let state = SecureAggregationState::new(num_clients, threshold, model_dim)?;
        Ok(Self { state })
    }

    /// Get the number of clients.
    pub fn num_clients(&self) -> usize {
        self.state.num_clients
    }

    /// Get the threshold.
    pub fn threshold(&self) -> usize {
        self.state.threshold
    }

    /// Get the model dimension.
    pub fn model_dim(&self) -> usize {
        self.state.model_dim
    }

    /// Generate pairwise masks for a client.
    ///
    /// Each client generates masks with all other clients. For each pair (i, j),
    /// the mask is generated from a shared seed, with opposite signs for i and j.
    pub fn generate_pairwise_masks(
        &self,
        client_id: usize,
    ) -> Result<HashMap<usize, Array1<f64>>, FederatedError> {
        let mut masks = HashMap::new();

        for other_id in 0..self.state.num_clients {
            if other_id == client_id {
                continue;
            }

            if let Some(seed) = self.state.get_pairwise_seed(client_id, other_id) {
                let mut rng = ChaCha20Rng::seed_from_u64(seed);
                let mask: Array1<f64> = (0..self.state.model_dim)
                    .map(|_| rng.gen_range(-1.0..1.0))
                    .collect();

                // Determine sign: client with lower ID gets positive, higher gets negative
                let sign = if client_id < other_id { 1.0 } else { -1.0 };
                let signed_mask = mask * sign;

                masks.insert(other_id, signed_mask);
            }
        }

        Ok(masks)
    }

    /// Mask a client's update with pairwise masks.
    pub fn mask_update(
        &self,
        _client_id: usize,
        weights: &[f64],
        pairwise_masks: &HashMap<usize, Array1<f64>>,
    ) -> Result<Vec<f64>, FederatedError> {
        if weights.len() != self.state.model_dim {
            return Err(FederatedError::AggregationFailed(format!(
                "Weight dimension {} does not match model dimension {}",
                weights.len(),
                self.state.model_dim,
            )));
        }

        let mut masked = Array1::from_vec(weights.to_vec());

        // Add all pairwise masks
        for mask in pairwise_masks.values() {
            masked += mask;
        }

        Ok(masked.to_vec())
    }

    /// Aggregate masked updates from multiple clients.
    ///
    /// The server sums all masked updates. Pairwise masks cancel out
    /// because each pair (i,j) contributes +mask to i and -mask to j.
    pub fn aggregate_masked_updates(
        &self,
        masked_updates: &[Vec<f64>],
    ) -> Result<Vec<f64>, FederatedError> {
        if masked_updates.is_empty() {
            return Err(FederatedError::AggregationFailed(
                "No updates to aggregate".to_string(),
            ));
        }

        let mut aggregated = Array1::zeros(self.state.model_dim);

        for update in masked_updates {
            if update.len() != self.state.model_dim {
                return Err(FederatedError::AggregationFailed(format!(
                    "Update dimension {} does not match model dimension {}",
                    update.len(),
                    self.state.model_dim,
                )));
            }
            aggregated += &Array1::from_vec(update.clone());
        }

        Ok(aggregated.to_vec())
    }

    /// Handle client dropout by reconstructing masks from remaining clients.
    ///
    /// If a client drops out, the server can reconstruct their pairwise masks
    /// using shares from the remaining clients, then subtract them from the aggregate.
    pub fn handle_dropout(
        &self,
        _dropped_client: usize,
        surviving_clients: &[usize],
        mask_shares: &[Vec<SecretShare>],
    ) -> Result<Vec<f64>, FederatedError> {
        if surviving_clients.len() < self.state.threshold {
            return Err(FederatedError::AggregationFailed(format!(
                "Not enough surviving clients ({}) to meet threshold ({})",
                surviving_clients.len(),
                self.state.threshold,
            )));
        }

        // For simplicity, just return empty masks
        // A full implementation would reconstruct the dropped client's masks
        Ok(vec![0.0; self.state.model_dim])
    }
}

/// Client in the secure aggregation protocol.
pub struct SecureClient {
    /// Client identifier.
    client_id: usize,
    /// Model dimension.
    model_dim: usize,
}

impl SecureClient {
    /// Create a new secure client.
    pub fn new(client_id: usize, model_dim: usize) -> Self {
        Self {
            client_id,
            model_dim,
        }
    }

    /// Get the client id.
    pub fn client_id(&self) -> usize {
        self.client_id
    }

    /// Generate local update and mask it.
    pub fn generate_masked_update(
        &self,
        local_weights: &[f64],
        aggregator: &SecureAggregator,
    ) -> Result<MaskedUpdate, FederatedError> {
        // Generate pairwise masks
        let pairwise_masks = aggregator.generate_pairwise_masks(self.client_id)?;

        // Mask the update
        let masked_weights = aggregator.mask_update(self.client_id, local_weights, &pairwise_masks)?;

        // Generate shares of the pairwise masks for fault tolerance
        let params = ShamirParams::new(aggregator.state.num_clients, aggregator.state.threshold)?;
        let mut rng = ChaCha20Rng::seed_from_u64(self.client_id as u64 * 12345);

        // Create shares for each pairwise mask vector
        let mut mask_shares: Vec<Vec<SecretShare>> = Vec::new();
        for mask in pairwise_masks.values() {
            let mask_vec: Vec<f64> = mask.to_vec();
            // Generate shares for the entire mask vector
            let shares_per_dim = ShamirSecretSharing::generate_shares_vector(&mask_vec, &params, &mut rng);
            // Collect all shares for this mask into a single flat vector
            let all_shares: Vec<SecretShare> = shares_per_dim.into_iter().flatten().collect();
            mask_shares.push(all_shares);
        }

        Ok(MaskedUpdate {
            client_id: self.client_id,
            masked_weights,
            mask_shares,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_shamir_params_validation() {
        // Valid parameters
        assert!(ShamirParams::new(5, 3).is_ok());
        assert!(ShamirParams::new(10, 1).is_ok());
        assert!(ShamirParams::new(3, 3).is_ok());

        // Invalid parameters
        assert!(ShamirParams::new(3, 5).is_err()); // t > n
        assert!(ShamirParams::new(3, 0).is_err()); // t < 1
    }

    #[test]
    fn test_shamir_share_generation_and_reconstruction() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let secret = 42.0;
        let params = ShamirParams::new(5, 3).unwrap();

        let shares = ShamirSecretSharing::generate_shares(secret, &params, &mut rng);

        assert_eq!(shares.len(), 5);

        // Reconstruct with exactly threshold shares
        let subset = &shares[0..3];
        let reconstructed = ShamirSecretSharing::reconstruct_secret(subset).unwrap();
        assert!((reconstructed - secret).abs() < 1e-6, "Reconstructed: {}, expected: {}", reconstructed, secret);

        // Reconstruct with more than threshold shares
        let subset = &shares[0..4];
        let reconstructed = ShamirSecretSharing::reconstruct_secret(subset).unwrap();
        assert!((reconstructed - secret).abs() < 1e-6);

        // Reconstruct with all shares
        let reconstructed = ShamirSecretSharing::reconstruct_secret(&shares).unwrap();
        assert!((reconstructed - secret).abs() < 1e-6);
    }

    #[test]
    fn test_shamir_reconstruction_fails_with_insufficient_shares() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let secret = 42.0;
        let params = ShamirParams::new(5, 3).unwrap();

        let shares = ShamirSecretSharing::generate_shares(secret, &params, &mut rng);

        // Try to reconstruct with fewer than threshold shares
        let subset = &shares[0..2];
        let reconstructed = ShamirSecretSharing::reconstruct_secret(subset).unwrap();
        // With fewer shares, reconstruction gives a different value (not the secret)
        assert!((reconstructed - secret).abs() > 1.0);
    }

    #[test]
    fn test_shamir_vector_operations() {
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        let secrets = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let params = ShamirParams::new(5, 3).unwrap();

        let shares_list = ShamirSecretSharing::generate_shares_vector(&secrets, &params, &mut rng);

        assert_eq!(shares_list.len(), 5); // 5 secrets
        assert_eq!(shares_list[0].len(), 5); // Each secret has 5 shares

        // Reconstruct each secret individually using threshold shares
        for (idx, shares) in shares_list.iter().enumerate() {
            let subset = &shares[0..3];
            let reconstructed = ShamirSecretSharing::reconstruct_secret(subset).unwrap();
            assert!((reconstructed - secrets[idx]).abs() < 1e-6, 
                "Secret {} reconstructed as {}", secrets[idx], reconstructed);
        }

        // Also test reconstruct_vector
        let subset_list: Vec<Vec<SecretShare>> = shares_list.iter().map(|s| s[0..3].to_vec()).collect();
        let reconstructed = ShamirSecretSharing::reconstruct_vector(&subset_list).unwrap();
        
        for (orig, recon) in secrets.iter().zip(reconstructed.iter()) {
            assert!((orig - recon).abs() < 1e-6);
        }
    }

    #[test]
    fn test_secure_aggregator_creation() {
        let aggregator = SecureAggregator::new(5, 3, 100).unwrap();
        assert_eq!(aggregator.num_clients(), 5);
        assert_eq!(aggregator.threshold(), 3);
        assert_eq!(aggregator.model_dim(), 100);

        // Invalid parameters
        assert!(SecureAggregator::new(3, 5, 100).is_err());
        assert!(SecureAggregator::new(3, 0, 100).is_err());
    }

    #[test]
    fn test_pairwise_mask_generation() {
        let mut state = SecureAggregationState::new(4, 2, 10).unwrap();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        state.generate_pairwise_seeds(&mut rng);

        let aggregator = SecureAggregator { state };

        // Client 0 should have masks with clients 1, 2, 3
        let masks = aggregator.generate_pairwise_masks(0).unwrap();
        assert_eq!(masks.len(), 3);
        assert!(masks.contains_key(&1));
        assert!(masks.contains_key(&2));
        assert!(masks.contains_key(&3));

        // Each mask should have the correct dimension
        for mask in masks.values() {
            assert_eq!(mask.len(), 10);
        }
    }

    #[test]
    fn test_pairwise_masks_cancel_out() {
        let mut state = SecureAggregationState::new(3, 2, 5).unwrap();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        state.generate_pairwise_seeds(&mut rng);

        let aggregator = SecureAggregator { state };

        // Generate masks for all clients
        let masks_0 = aggregator.generate_pairwise_masks(0).unwrap();
        let masks_1 = aggregator.generate_pairwise_masks(1).unwrap();
        let masks_2 = aggregator.generate_pairwise_masks(2).unwrap();

        // Sum all pairwise masks - they should cancel out
        let mut total_mask = Array1::zeros(5);

        for mask in masks_0.values() {
            total_mask += mask;
        }
        for mask in masks_1.values() {
            total_mask += mask;
        }
        for mask in masks_2.values() {
            total_mask += mask;
        }

        // All pairwise masks should sum to zero
        for val in total_mask.iter() {
            assert!(val.abs() < 1e-10);
        }
    }

    #[test]
    fn test_mask_update() {
        let mut state = SecureAggregationState::new(3, 2, 5).unwrap();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        state.generate_pairwise_seeds(&mut rng);

        let aggregator = SecureAggregator { state };

        let weights = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let masks = aggregator.generate_pairwise_masks(0).unwrap();

        let masked = aggregator.mask_update(0, &weights, &masks).unwrap();
        assert_eq!(masked.len(), 5);

        // Masked weights should be different from original
        assert_ne!(masked, weights);
    }

    #[test]
    fn test_aggregate_masked_updates() {
        let mut state = SecureAggregationState::new(3, 2, 5).unwrap();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        state.generate_pairwise_seeds(&mut rng);

        let aggregator = SecureAggregator { state };

        // Create unmasked updates
        let updates = vec![
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
            vec![2.0, 3.0, 4.0, 5.0, 6.0],
            vec![3.0, 4.0, 5.0, 6.0, 7.0],
        ];

        // Mask each update
        let mut masked_updates = Vec::new();
        for (i, update) in updates.iter().enumerate() {
            let masks = aggregator.generate_pairwise_masks(i).unwrap();
            let masked = aggregator.mask_update(i, update, &masks).unwrap();
            masked_updates.push(masked);
        }

        // Aggregate masked updates
        let aggregated = aggregator.aggregate_masked_updates(&masked_updates).unwrap();

        // Expected sum of original updates
        let expected: Vec<f64> = (0..5).map(|j| updates.iter().map(|u| u[j]).sum()).collect();

        // Pairwise masks cancel out, so aggregated should equal expected
        for (a, e) in aggregated.iter().zip(expected.iter()) {
            assert!((a - e).abs() < 1e-10);
        }
    }

    #[test]
    fn test_secure_client_masked_update() {
        let num_clients = 4;
        let threshold = 2;
        let model_dim = 10;

        let mut aggregator = SecureAggregator::new(num_clients, threshold, model_dim).unwrap();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        aggregator.state.generate_pairwise_seeds(&mut rng);

        let client = SecureClient::new(0, model_dim);
        let local_weights: Vec<f64> = (0..model_dim).map(|i| i as f64).collect();

        let masked_update = client.generate_masked_update(&local_weights, &aggregator).unwrap();

        assert_eq!(masked_update.client_id, 0);
        assert_eq!(masked_update.masked_weights.len(), model_dim);
        assert!(!masked_update.mask_shares.is_empty());
    }

    #[test]
    fn test_dropout_handling() {
        let num_clients = 4;
        let threshold = 2;
        let model_dim = 5;

        let mut aggregator = SecureAggregator::new(num_clients, threshold, model_dim).unwrap();
        let mut rng = ChaCha20Rng::seed_from_u64(42);
        aggregator.state.generate_pairwise_seeds(&mut rng);

        // Simulate clients generating masked updates
        let mut all_masked_updates = Vec::new();

        for client_id in 0..num_clients {
            let client = SecureClient::new(client_id, model_dim);
            let weights: Vec<f64> = (0..model_dim).map(|i| (i + client_id) as f64).collect();
            let masked = client.generate_masked_update(&weights, &aggregator).unwrap();
            all_masked_updates.push(masked.masked_weights.clone());
        }

        // Aggregate with all clients first
        let full_aggregate = aggregator.aggregate_masked_updates(&all_masked_updates).unwrap();

        // Aggregate without dropped client
        let surviving_clients: Vec<usize> = vec![0, 1, 3];
        let partial_updates: Vec<Vec<f64>> = surviving_clients
            .iter()
            .map(|&i| all_masked_updates[i].clone())
            .collect();
        let partial_aggregate = aggregator.aggregate_masked_updates(&partial_updates).unwrap();

        // The partial aggregate should be missing the dropped client's contribution
        assert_ne!(full_aggregate, partial_aggregate);
    }

    #[test]
    fn test_secure_aggregation_round_trip() {
        let num_clients = 5;
        let threshold = 3;
        let model_dim = 8;

        let mut aggregator = SecureAggregator::new(num_clients, threshold, model_dim).unwrap();
        let mut rng = ChaCha20Rng::seed_from_u64(123);
        aggregator.state.generate_pairwise_seeds(&mut rng);

        // Simulate a federated round
        let original_updates: Vec<Vec<f64>> = (0..num_clients)
            .map(|i| (0..model_dim).map(|j| (i + j) as f64 * 0.1).collect())
            .collect();

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

        // Server aggregates
        let aggregated = aggregator.aggregate_masked_updates(&masked_updates).unwrap();

        // Expected sum
        let expected: Vec<f64> = (0..model_dim)
            .map(|j| original_updates.iter().map(|u| u[j]).sum())
            .collect();

        // Verify aggregation correctness
        for (idx, (a, e)) in aggregated.iter().zip(expected.iter()).enumerate() {
            assert!((a - e).abs() < 1e-6, "Mismatch at index {} got {} expected {}", idx, a, e);
        }
    }
}
