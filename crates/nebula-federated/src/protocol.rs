//! Communication protocol traits for client-server interaction

use crate::types::{ClientUpdate, ModelParameters, RoundId, RoundStatus, FederatedError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Request model update from client
    RequestUpdate {
        round_id: RoundId,
        global_model: ModelParameters,
        config: UpdateConfig,
    },
    /// Acknowledge receipt of update
    Acknowledge {
        round_id: RoundId,
        client_id: String,
        accepted: bool,
        message: Option<String>,
    },
    /// Notify client of round status
    RoundStatus {
        round_id: RoundId,
        status: RoundStatus,
        global_model: Option<ModelParameters>,
    },
    /// Server shutdown notice
    Shutdown {
        reason: String,
    },
}

/// Configuration for a client update request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Number of local epochs to train
    pub epochs: usize,
    /// Batch size for training
    pub batch_size: usize,
    /// Learning rate
    pub learning_rate: f64,
    /// Timeout in milliseconds for the update
    pub timeout_ms: u64,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            epochs: 5,
            batch_size: 32,
            learning_rate: 0.01,
            timeout_ms: 300000, // 5 minutes
        }
    }
}

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Client registration with the server
    Register {
        client_id: String,
        capabilities: ClientCapabilities,
    },
    /// Submit model update
    SubmitUpdate {
        update: ClientUpdate,
    },
    /// Heartbeat to indicate client is alive
    Heartbeat {
        client_id: String,
        timestamp: u64,
        metrics: Option<std::collections::HashMap<String, f64>>,
    },
    /// Client disconnect notice
    Disconnect {
        client_id: String,
        reason: Option<String>,
    },
}

/// Capabilities advertised by a client
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    /// Maximum model size the client can handle (in parameters)
    pub max_model_size: Option<usize>,
    /// Supported aggregation methods
    pub supported_aggregations: Vec<String>,
    /// Whether client supports differential privacy
    pub supports_privacy: bool,
    /// Compute resources (CPU cores, GPU memory in MB)
    pub compute_resources: Option<ComputeResources>,
}

/// Compute resource information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeResources {
    /// Number of CPU cores
    pub cpu_cores: usize,
    /// GPU memory in MB (if available)
    pub gpu_memory_mb: Option<u64>,
    /// Available RAM in MB
    pub ram_mb: u64,
}

/// Trait for server-side operations
#[async_trait]
pub trait FederatedServer: Send + Sync {
    /// Start the server
    async fn start(&self) -> Result<(), FederatedError>;

    /// Stop the server gracefully
    async fn stop(&self) -> Result<(), FederatedError>;

    /// Register a new client
    async fn register_client(
        &self,
        client_id: String,
        capabilities: ClientCapabilities,
    ) -> Result<bool, FederatedError>;

    /// Unregister a client
    async fn unregister_client(&self, client_id: &str) -> Result<(), FederatedError>;

    /// Start a new aggregation round
    async fn start_round(&self) -> Result<RoundId, FederatedError>;

    /// Get the current round ID
    async fn current_round(&self) -> Option<RoundId>;

    /// Get round status
    async fn get_round_status(&self, round_id: RoundId) -> Result<RoundStatus, FederatedError>;

    /// Process a client update
    async fn process_update(
        &self,
        update: ClientUpdate,
    ) -> Result<bool, FederatedError>;

    /// Get the global model
    async fn get_global_model(&self) -> Result<ModelParameters, FederatedError>;

    /// Get the number of active clients
    async fn num_active_clients(&self) -> usize;

    /// Get connected client IDs
    async fn get_client_ids(&self) -> Vec<String>;
}

/// Trait for client-side operations
#[async_trait]
pub trait FederatedClient: Send + Sync {
    /// Get the client ID
    fn client_id(&self) -> &str;

    /// Connect to the server
    async fn connect(&self, server_addr: &str) -> Result<(), FederatedError>;

    /// Disconnect from the server
    async fn disconnect(&self) -> Result<(), FederatedError>;

    /// Check if connected
    async fn is_connected(&self) -> bool;

    /// Receive messages from server
    async fn recv_message(&self) -> Result<ServerMessage, FederatedError>;

    /// Send messages to server
    async fn send_message(&self, msg: ClientMessage) -> Result<(), FederatedError>;

    /// Train on local data and produce an update
    async fn train_local(
        &self,
        global_model: &ModelParameters,
        config: &UpdateConfig,
    ) -> Result<ClientUpdate, FederatedError>;

    /// Get local data size
    async fn get_data_size(&self) -> Result<usize, FederatedError>;
}

/// Protocol handler for message serialization/deserialization
pub trait ProtocolHandler: Send + Sync {
    /// Serialize a server message to bytes
    fn serialize_server_msg(&self, msg: &ServerMessage) -> Result<Vec<u8>, FederatedError>;

    /// Deserialize a server message from bytes
    fn deserialize_server_msg(&self, bytes: &[u8]) -> Result<ServerMessage, FederatedError>;

    /// Serialize a client message to bytes
    fn serialize_client_msg(&self, msg: &ClientMessage) -> Result<Vec<u8>, FederatedError>;

    /// Deserialize a client message from bytes
    fn deserialize_client_msg(&self, bytes: &[u8]) -> Result<ClientMessage, FederatedError>;
}

/// JSON-based protocol handler
#[derive(Debug, Clone, Default)]
pub struct JsonProtocol;

impl ProtocolHandler for JsonProtocol {
    fn serialize_server_msg(&self, msg: &ServerMessage) -> Result<Vec<u8>, FederatedError> {
        serde_json::to_vec(msg).map_err(FederatedError::Serialization)
    }

    fn deserialize_server_msg(&self, bytes: &[u8]) -> Result<ServerMessage, FederatedError> {
        serde_json::from_slice(bytes).map_err(FederatedError::Serialization)
    }

    fn serialize_client_msg(&self, msg: &ClientMessage) -> Result<Vec<u8>, FederatedError> {
        serde_json::to_vec(msg).map_err(FederatedError::Serialization)
    }

    fn deserialize_client_msg(&self, bytes: &[u8]) -> Result<ClientMessage, FederatedError> {
        serde_json::from_slice(bytes).map_err(FederatedError::Serialization)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_protocol_serialization() {
        let protocol = JsonProtocol;

        let server_msg = ServerMessage::RequestUpdate {
            round_id: 1,
            global_model: ModelParameters::new(vec![1.0, 2.0, 3.0]),
            config: UpdateConfig::default(),
        };

        let bytes = protocol.serialize_server_msg(&server_msg).unwrap();
        let deserialized = protocol.deserialize_server_msg(&bytes).unwrap();

        match deserialized {
            ServerMessage::RequestUpdate { round_id, .. } => {
                assert_eq!(round_id, 1);
            }
            _ => panic!("Expected RequestUpdate"),
        }
    }

    #[test]
    fn test_client_message_serialization() {
        let protocol = JsonProtocol;

        let client_msg = ClientMessage::Register {
            client_id: "client_1".to_string(),
            capabilities: ClientCapabilities::default(),
        };

        let bytes = protocol.serialize_client_msg(&client_msg).unwrap();
        let deserialized = protocol.deserialize_client_msg(&bytes).unwrap();

        match deserialized {
            ClientMessage::Register { client_id, .. } => {
                assert_eq!(client_id, "client_1");
            }
            _ => panic!("Expected Register"),
        }
    }

    #[test]
    fn test_heartbeat_message() {
        let protocol = JsonProtocol;

        let mut metrics = HashMap::new();
        metrics.insert("cpu_usage".to_string(), 45.2);
        metrics.insert("memory_usage".to_string(), 67.8);

        let heartbeat = ClientMessage::Heartbeat {
            client_id: "client_1".to_string(),
            timestamp: 1234567890,
            metrics: Some(metrics),
        };

        let bytes = protocol.serialize_client_msg(&heartbeat).unwrap();
        let deserialized = protocol.deserialize_client_msg(&bytes).unwrap();

        match deserialized {
            ClientMessage::Heartbeat { client_id, timestamp, metrics: _ } => {
                assert_eq!(client_id, "client_1");
                assert_eq!(timestamp, 1234567890);
            }
            _ => panic!("Expected Heartbeat"),
        }
    }
}
