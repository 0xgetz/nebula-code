//! Client implementation for federated learning

use crate::protocol::{
    ClientCapabilities, ClientMessage, FederatedClient, ProtocolHandler, ServerMessage, UpdateConfig,
};
use crate::types::{ClientUpdate, FederatedError, ModelParameters};
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

/// In-memory federated learning client
pub struct FederatedClientImpl {
    /// Unique client identifier
    client_id: String,
    /// Whether client is connected
    connected: Arc<RwLock<bool>>,
    /// Sender for messages TO the server
    to_server_tx: Option<mpsc::UnboundedSender<ClientMessage>>,
    /// Receiver for messages FROM the server
    from_server_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<ServerMessage>>>>,
    /// Local data size (simulated)
    local_data_size: usize,
    /// Protocol handler
    protocol: Box<dyn ProtocolHandler>,
}

impl FederatedClientImpl {
    /// Create a new client with the given ID
    pub fn new(client_id: String) -> Self {
        Self {
            client_id,
            connected: Arc::new(RwLock::new(false)),
            to_server_tx: None,
            from_server_rx: Arc::new(RwLock::new(None)),
            local_data_size: 0,
            protocol: Box::new(crate::protocol::JsonProtocol),
        }
    }

    /// Create a client with simulated local data
    pub fn with_data(client_id: String, data_size: usize) -> Self {
        Self {
            client_id,
            connected: Arc::new(RwLock::new(false)),
            to_server_tx: None,
            from_server_rx: Arc::new(RwLock::new(None)),
            local_data_size: data_size,
            protocol: Box::new(crate::protocol::JsonProtocol),
        }
    }

    /// Set the protocol handler
    pub fn set_protocol<P: ProtocolHandler + 'static>(&mut self, protocol: P) {
        self.protocol = Box::new(protocol);
    }

    /// Get client capabilities
    pub fn capabilities(&self) -> ClientCapabilities {
        ClientCapabilities {
            max_model_size: Some(1_000_000),
            supported_aggregations: vec!["FedAvg".to_string(), "SimpleAverage".to_string()],
            supports_privacy: true,
            compute_resources: None,
        }
    }

    /// Set local data size
    pub fn set_data_size(&mut self, size: usize) {
        self.local_data_size = size;
    }

    /// Simulate receiving a message from server (for testing)
    pub async fn simulate_server_message(&self, msg: ServerMessage) {
        // This would require a separate channel for simulation
        // For now, this is a no-op in tests - we use MockClient instead
        let _ = msg;
    }
}

#[async_trait::async_trait]
impl FederatedClient for FederatedClientImpl {
    fn client_id(&self) -> &str {
        &self.client_id
    }

    async fn connect(&self, _server_addr: &str) -> Result<(), FederatedError> {
        // In a real implementation, we'd establish a network connection
        // For now, just mark as connected
        let mut connected = self.connected.write().await;
        *connected = true;
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), FederatedError> {
        // Send disconnect message if connected
        if let Some(tx) = &self.to_server_tx {
            let msg = ClientMessage::Disconnect {
                client_id: self.client_id.clone(),
                reason: Some("Client disconnecting".to_string()),
            };
            let _ = tx.send(msg);
        }

        let mut connected = self.connected.write().await;
        *connected = false;

        Ok(())
    }

    async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    async fn recv_message(&self) -> Result<ServerMessage, FederatedError> {
        let mut rx_guard = self.from_server_rx.write().await;
        if let Some(rx) = rx_guard.as_mut() {
            rx.recv()
                .await
                .ok_or(FederatedError::Network("Channel closed".to_string()))
        } else {
            Err(FederatedError::Network("Not connected".to_string()))
        }
    }

    async fn send_message(&self, msg: ClientMessage) -> Result<(), FederatedError> {
        if let Some(tx) = &self.to_server_tx {
            tx.send(msg)
                .map_err(|e| FederatedError::Network(e.to_string()))?
        }
        Ok(())
    }

    async fn train_local(
        &self,
        global_model: &ModelParameters,
        config: &UpdateConfig,
    ) -> Result<ClientUpdate, FederatedError> {
        // Simulate local training by adding some noise to the global model
        // In a real implementation, this would involve actual ML training

        let mut rng = rand::thread_rng();
        let normal = rand_distr::Normal::new(0.0, config.learning_rate).unwrap();

        let updated_values: Vec<f64> = global_model
            .values
            .iter()
            .map(|&v| v + rand_distr::Distribution::sample(&normal, &mut rng))
            .collect();

        let updated_params = ModelParameters::new(updated_values);

        // Create update (round_id will be set by the caller)
        let update = ClientUpdate::new(
            self.client_id.clone(),
            0, // round_id will be set later
            updated_params,
            self.local_data_size,
        );

        // Add some simulated metrics
        let update = update
            .with_metric("training_loss".to_string(), 0.1)
            .with_metric("training_accuracy".to_string(), 0.95);

        Ok(update)
    }

    async fn get_data_size(&self) -> Result<usize, FederatedError> {
        Ok(self.local_data_size)
    }
}

/// A simple client that can be used for testing without network
pub struct MockClient {
    client_id: String,
    local_data_size: usize,
    updates_sent: Arc<RwLock<Vec<ClientUpdate>>>,
    messages_received: Arc<RwLock<Vec<ServerMessage>>>,
}

impl MockClient {
    pub fn new(client_id: String, data_size: usize) -> Self {
        Self {
            client_id,
            local_data_size: data_size,
            updates_sent: Arc::new(RwLock::new(Vec::new())),
            messages_received: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn get_updates_sent(&self) -> Vec<ClientUpdate> {
        self.updates_sent.read().await.clone()
    }

    pub async fn get_messages_received(&self) -> Vec<ServerMessage> {
        self.messages_received.read().await.clone()
    }

    pub async fn simulate_receive(&self, msg: ServerMessage) {
        self.messages_received.write().await.push(msg);
    }
}

#[async_trait::async_trait]
impl FederatedClient for MockClient {
    fn client_id(&self) -> &str {
        &self.client_id
    }

    async fn connect(&self, _server_addr: &str) -> Result<(), FederatedError> {
        Ok(())
    }

    async fn disconnect(&self) -> Result<(), FederatedError> {
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        true
    }

    async fn recv_message(&self) -> Result<ServerMessage, FederatedError> {
        let messages = self.messages_received.read().await;
        messages
            .first()
            .cloned()
            .ok_or(FederatedError::Network("No messages".to_string()))
    }

    async fn send_message(&self, _msg: ClientMessage) -> Result<(), FederatedError> {
        Ok(())
    }

    async fn train_local(
        &self,
        global_model: &ModelParameters,
        _config: &UpdateConfig,
    ) -> Result<ClientUpdate, FederatedError> {
        // Simulate training by slightly modifying the model
        let updated_values: Vec<f64> = global_model
            .values
            .iter()
            .map(|&v| v + 0.01)
            .collect();

        let params = ModelParameters::new(updated_values);
        let mut update = ClientUpdate::new(
            self.client_id.clone(),
            0,
            params,
            self.local_data_size,
        );

        update = update.with_metric("mock_accuracy".to_string(), 0.9);

        self.updates_sent.write().await.push(update.clone());

        Ok(update)
    }

    async fn get_data_size(&self) -> Result<usize, FederatedError> {
        Ok(self.local_data_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = FederatedClientImpl::new("test_client".to_string());
        assert_eq!(client.client_id(), "test_client");
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_mock_client_training() {
        let client = MockClient::new("mock_client".to_string(), 100);
        let model = ModelParameters::new(vec![1.0, 2.0, 3.0]);
        let config = UpdateConfig::default();

        let update = client.train_local(&model, &config).await.unwrap();

        assert_eq!(update.client_id, "mock_client");
        assert_eq!(update.num_samples, 100);
        assert_eq!(update.parameters.values.len(), 3);

        // Check that values were modified
        for (orig, updated) in model.values.iter().zip(update.parameters.values.iter()) {
            assert!((updated - orig).abs() > 0.0);
        }
    }

    #[tokio::test]
    async fn test_client_connection() {
        let client = FederatedClientImpl::new("test_client".to_string());

        assert!(!client.is_connected().await);

        // Connect (with dummy address)
        let result = client.connect("localhost:8080").await;
        assert!(result.is_ok());
        assert!(client.is_connected().await);

        // Disconnect
        let result = client.disconnect().await;
        assert!(result.is_ok());
        assert!(!client.is_connected().await);
    }
}
