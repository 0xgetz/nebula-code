//! Server implementation for federated learning

use crate::protocol::{ClientCapabilities, FederatedServer};
use crate::types::{
    AggregationMethod, AggregationRound, ClientUpdate, FederatedConfig,
    FederatedError, ModelParameters, RoundId, RoundStatus,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// In-memory federated learning server
pub struct FederatedServerImpl {
    /// Server configuration
    config: FederatedConfig,
    /// Connected clients
    clients: Arc<RwLock<HashMap<String, ClientCapabilities>>>,
    /// Current round
    current_round: Arc<AtomicU64>,
    /// Rounds storage
    rounds: Arc<RwLock<HashMap<RoundId, AggregationRound>>>,
    /// Global model
    global_model: Arc<RwLock<ModelParameters>>,
    /// Whether server is running
    running: Arc<RwLock<bool>>,
}

impl FederatedServerImpl {
    /// Create a new server with the given configuration
    pub fn new(config: FederatedConfig, initial_model: ModelParameters) -> Self {
        Self {
            config,
            clients: Arc::new(RwLock::new(HashMap::new())),
            current_round: Arc::new(AtomicU64::new(0)),
            rounds: Arc::new(RwLock::new(HashMap::new())),
            global_model: Arc::new(RwLock::new(initial_model)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Get the current global model
    pub async fn global_model(&self) -> ModelParameters {
        self.global_model.read().await.clone()
    }

    /// Get a round by ID
    pub async fn get_round(&self, round_id: RoundId) -> Option<AggregationRound> {
        self.rounds.read().await.get(&round_id).cloned()
    }

    /// Get all rounds
    pub async fn get_all_rounds(&self) -> Vec<AggregationRound> {
        self.rounds.read().await.values().cloned().collect()
    }

    /// Sample clients for the next round
    pub async fn sample_clients(&self) -> Vec<String> {
        let clients = self.clients.read().await;
        let num_to_sample = ((clients.len() as f64) * self.config.client_fraction) as usize;
        let num_to_sample = num_to_sample.max(self.config.aggregation.min_clients);
        let num_to_sample = num_to_sample.min(clients.len());

        // Simple random sampling
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let client_ids: Vec<_> = clients.keys().cloned().collect();
        client_ids
            .choose_multiple(&mut rng, num_to_sample)
            .cloned()
            .collect()
    }

    /// Aggregate updates using the configured method
    pub async fn aggregate_updates(
        &self,
        round: &mut AggregationRound,
    ) -> Result<ModelParameters, FederatedError> {
        if round.client_updates.is_empty() {
            return Err(FederatedError::InsufficientClients {
                needed: 1,
                available: 0,
            });
        }

        match round.config.method {
            AggregationMethod::FedAvg => self.aggregate_fedavg(round),
            AggregationMethod::SimpleAverage => self.aggregate_simple_average(round),
            AggregationMethod::WeightedAverage => self.aggregate_weighted_average(round),
        }
    }

    /// Federated Averaging (weighted by sample count)
    fn aggregate_fedavg(&self, round: &AggregationRound) -> Result<ModelParameters, FederatedError> {
        let total_samples: usize = round.client_updates.iter().map(|u| u.num_samples).sum();

        if total_samples == 0 {
            return Err(FederatedError::AggregationFailed(
                "Total samples is zero".to_string(),
            ));
        }

        let mut aggregated = round.client_updates[0]
            .parameters
            .scale(round.client_updates[0].num_samples as f64 / total_samples as f64);

        for update in round.client_updates.iter().skip(1) {
            let weighted = update.parameters.scale(update.num_samples as f64 / total_samples as f64);
            aggregated = aggregated.add(&weighted)?;
        }

        Ok(aggregated)
    }

    /// Simple average (equal weights)
    fn aggregate_simple_average(
        &self,
        round: &AggregationRound,
    ) -> Result<ModelParameters, FederatedError> {
        let num_updates = round.client_updates.len();
        if num_updates == 0 {
            return Err(FederatedError::AggregationFailed(
                "No updates to aggregate".to_string(),
            ));
        }

        let weight = 1.0 / num_updates as f64;
        let mut aggregated = round.client_updates[0].parameters.scale(weight);

        for update in round.client_updates.iter().skip(1) {
            let weighted = update.parameters.scale(weight);
            aggregated = aggregated.add(&weighted)?;
        }

        Ok(aggregated)
    }

    /// Weighted average with custom weights (currently uses sample count as weight)
    fn aggregate_weighted_average(
        &self,
        round: &AggregationRound,
    ) -> Result<ModelParameters, FederatedError> {
        // For now, same as FedAvg but could be extended with custom weights
        self.aggregate_fedavg(round)
    }
}

#[async_trait::async_trait]
impl FederatedServer for FederatedServerImpl {
    async fn start(&self) -> Result<(), FederatedError> {
        let mut running = self.running.write().await;
        *running = true;
        Ok(())
    }

    async fn stop(&self) -> Result<(), FederatedError> {
        let mut running = self.running.write().await;
        *running = false;
        Ok(())
    }

    async fn register_client(
        &self,
        client_id: String,
        capabilities: ClientCapabilities,
    ) -> Result<bool, FederatedError> {
        let mut clients = self.clients.write().await;
        let is_new = !clients.contains_key(&client_id);
        clients.insert(client_id, capabilities);
        Ok(is_new)
    }

    async fn unregister_client(&self, client_id: &str) -> Result<(), FederatedError> {
        let mut clients = self.clients.write().await;
        clients.remove(client_id);
        Ok(())
    }

    async fn start_round(&self) -> Result<RoundId, FederatedError> {
        let round_id = self.current_round.fetch_add(1, Ordering::SeqCst);
        let global_model = self.global_model.read().await.clone();

        let round = AggregationRound::new(
            round_id,
            global_model,
            self.config.aggregation.clone(),
        );

        // Sample clients for this round
        let _sampled_clients = self.sample_clients().await;
        // In a real implementation, we'd send RequestUpdate to these clients

        let mut rounds = self.rounds.write().await;
        rounds.insert(round_id, round);

        Ok(round_id)
    }

    async fn current_round(&self) -> Option<RoundId> {
        let current = self.current_round.load(Ordering::SeqCst);
        if current == 0 && self.rounds.read().await.is_empty() {
            None
        } else {
            Some(current.saturating_sub(1))
        }
    }

    async fn get_round_status(&self, round_id: RoundId) -> Result<RoundStatus, FederatedError> {
        let rounds = self.rounds.read().await;
        rounds
            .get(&round_id)
            .map(|r| r.status)
            .ok_or(FederatedError::RoundNotActive { round_id })
    }

    async fn process_update(
        &self,
        update: ClientUpdate,
    ) -> Result<bool, FederatedError> {
        let round_id = update.round_id;
        let mut rounds = self.rounds.write().await;

        if let Some(round) = rounds.get_mut(&round_id) {
            if round.status != RoundStatus::Active {
                return Err(FederatedError::RoundNotActive { round_id });
            }

            round.add_update(update);

            // Check if we have enough updates to aggregate
            if round.num_updates() >= round.config.min_clients {
                // Perform aggregation
                let new_global = self.aggregate_updates(round).await?;
                round.global_model = new_global.clone();
                round.complete();

                // Update global model
                let mut global_model = self.global_model.write().await;
                *global_model = new_global;

                return Ok(true);
            }

            Ok(true)
        } else {
            Err(FederatedError::RoundNotActive { round_id })
        }
    }

    async fn get_global_model(&self) -> Result<ModelParameters, FederatedError> {
        Ok(self.global_model.read().await.clone())
    }

    async fn num_active_clients(&self) -> usize {
        self.clients.read().await.len()
    }

    async fn get_client_ids(&self) -> Vec<String> {
        self.clients.read().await.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let config = FederatedConfig::new("test_job".to_string(), 10, 5);
        let model = ModelParameters::new(vec![0.0; 10]);
        let server = FederatedServerImpl::new(config, model);

        assert_eq!(server.num_active_clients().await, 0);
    }

    #[tokio::test]
    async fn test_client_registration() {
        let config = FederatedConfig::new("test_job".to_string(), 10, 5);
        let model = ModelParameters::new(vec![0.0; 10]);
        let server = FederatedServerImpl::new(config, model);

        let is_new = server
            .register_client("client_1".to_string(), ClientCapabilities::default())
            .await
            .unwrap();
        assert!(is_new);

        let is_new2 = server
            .register_client("client_1".to_string(), ClientCapabilities::default())
            .await
            .unwrap();
        assert!(!is_new2);

        assert_eq!(server.num_active_clients().await, 1);
    }

    #[tokio::test]
    async fn test_round_creation() {
        let config = FederatedConfig::new("test_job".to_string(), 10, 5);
        let model = ModelParameters::new(vec![0.0; 10]);
        let server = FederatedServerImpl::new(config, model);

        // Register some clients
        for i in 0..5 {
            server
                .register_client(format!("client_{}", i), ClientCapabilities::default())
                .await
                .unwrap();
        }

        let round_id = server.start_round().await.unwrap();
        assert_eq!(round_id, 0);

        let round = server.get_round(round_id).await.unwrap();
        assert_eq!(round.status, RoundStatus::Active);
    }

    #[tokio::test]
    async fn test_update_processing_and_aggregation() {
        let config = FederatedConfig::new("test_job".to_string(), 10, 5);
        let initial_model = ModelParameters::new(vec![0.0; 10]);
        let server = FederatedServerImpl::new(config, initial_model);

        // Register clients
        for i in 0..3 {
            server
                .register_client(format!("client_{}", i), ClientCapabilities::default())
                .await
                .unwrap();
        }

        // Start round
        let round_id = server.start_round().await.unwrap();

        // Submit updates from 3 clients
        for i in 0..3 {
            let params = ModelParameters::new(vec![1.0; 10]);
            let update = ClientUpdate::new(format!("client_{}", i), round_id, params, 100);
            server.process_update(update).await.unwrap();
        }

        // Check that round is complete
        let status = server.get_round_status(round_id).await.unwrap();
        assert_eq!(status, RoundStatus::Completed);

        // Check global model was updated
        let global_model = server.get_global_model().await.unwrap();
        assert_eq!(global_model.values, vec![1.0; 10]);
    }
}
