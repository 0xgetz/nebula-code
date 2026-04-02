//! Communication protocol definitions for inter-agent messaging.

use crate::types::AgentId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during communication.
#[derive(Debug, Error)]
pub enum CommunicationError {
    #[error("Failed to send message: {0}")]
    SendFailed(String),
    #[error("Failed to receive message: {0}")]
    ReceiveFailed(String),
    #[error("Message serialization error: {0}")]
    SerializationError(String),
    #[error("Connection lost: {0}")]
    ConnectionLost(String),
    #[error("Timeout: {0}")]
    Timeout(String),
}

/// Types of messages that can be exchanged between agents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Request for information or action.
    Request,
    /// Response to a request.
    Response,
    /// Notification of an event or state change.
    Notification,
    /// Task assignment from orchestrator.
    TaskAssign,
    /// Status update from an agent.
    StatusUpdate,
    /// Error report.
    Error,
    /// Heartbeat/keep-alive signal.
    Heartbeat,
    /// Subscription to a topic.
    Subscribe,
    /// Unsubscription from a topic.
    Unsubscribe,
    /// Publication to a topic.
    Publish,
}

/// A message exchanged between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for this message.
    pub id: String,
    /// Type of the message.
    pub message_type: MessageType,
    /// Sender agent ID.
    pub from: AgentId,
    /// Recipient agent ID (None for broadcasts).
    pub to: Option<AgentId>,
    /// Topic or channel for pub/sub messaging.
    pub topic: Option<String>,
    /// Message payload.
    pub payload: serde_json::Value,
    /// Optional correlation ID for request-response pairing.
    pub correlation_id: Option<String>,
    /// Timestamp when the message was created.
    #[serde(with = "chrono_serializer")]
    pub timestamp: std::time::SystemTime,
    /// Optional TTL in seconds.
    pub ttl_seconds: Option<u64>,
}

// Helper module for serializing SystemTime as Unix timestamp
mod chrono_serializer {
    use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).map_err(serde::ser::Error::custom)?;
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs(secs))
    }
}

impl Message {
    /// Creates a new message.
    pub fn new(
        message_type: MessageType,
        from: AgentId,
        to: Option<AgentId>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message_type,
            from,
            to,
            topic: None,
            payload,
            correlation_id: None,
            timestamp: std::time::SystemTime::now(),
            ttl_seconds: None,
        }
    }

    /// Creates a request message with a correlation ID.
    pub fn request(
        from: AgentId,
        to: AgentId,
        payload: serde_json::Value,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id: id.clone(),
            message_type: MessageType::Request,
            from,
            to: Some(to),
            topic: None,
            payload,
            correlation_id: Some(id),
            timestamp: std::time::SystemTime::now(),
            ttl_seconds: None,
        }
    }

    /// Creates a response message correlated to a request.
    pub fn response(
        from: AgentId,
        to: AgentId,
        correlation_id: String,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message_type: MessageType::Response,
            from,
            to: Some(to),
            topic: None,
            payload,
            correlation_id: Some(correlation_id),
            timestamp: std::time::SystemTime::now(),
            ttl_seconds: None,
        }
    }

    /// Creates a notification message.
    pub fn notification(
        from: AgentId,
        topic: Option<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message_type: MessageType::Notification,
            from,
            to: None,
            topic,
            payload,
            correlation_id: None,
            timestamp: std::time::SystemTime::now(),
            ttl_seconds: None,
        }
    }

    /// Sets the topic for pub/sub messaging.
    pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = Some(topic.into());
        self
    }

    /// Sets the TTL for the message.
    pub fn with_ttl(mut self, seconds: u64) -> Self {
        self.ttl_seconds = Some(seconds);
        self
    }

    /// Sets the correlation ID.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
}

/// Trait for implementing communication protocols.
#[async_trait::async_trait]
pub trait CommunicationProtocol: Send + Sync {
    /// Sends a message to the specified recipient.
    async fn send(&self, message: Message) -> Result<(), CommunicationError>;

    /// Receives the next message from the queue.
    async fn receive(&self) -> Result<Message, CommunicationError>;

    /// Subscribes to a topic.
    async fn subscribe(&self, topic: &str) -> Result<(), CommunicationError>;

    /// Unsubscribes from a topic.
    async fn unsubscribe(&self, topic: &str) -> Result<(), CommunicationError>;

    /// Publishes a message to a topic.
    async fn publish(&self, topic: &str, message: Message) -> Result<(), CommunicationError>;

    /// Broadcasts a message to all connected agents.
    async fn broadcast(&self, message: Message) -> Result<(), CommunicationError>;

    /// Checks if the protocol is connected and healthy.
    async fn is_connected(&self) -> bool;

    /// Closes the connection.
    async fn close(&self) -> Result<(), CommunicationError>;
}
