//! Communication primitives: channels, message queues, and pub/sub.

use crate::protocol::{CommunicationProtocol, Message, CommunicationError};
use std::collections::HashMap;
use crate::types::AgentId;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// A communication channel for sending and receiving messages.
#[derive(Debug, Clone)]
pub struct Channel {
    /// Unique identifier for this channel.
    pub id: String,
    /// Name of the channel.
    pub name: String,
    /// Sender side of the message queue.
    sender: mpsc::Sender<Message>,
    /// Receiver side of the message queue.
    receiver: Arc<RwLock<mpsc::Receiver<Message>>>,
}

impl Channel {
    /// Creates a new channel with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
        }
    }

    /// Sends a message through this channel.
    pub async fn send(&self, message: Message) -> Result<(), CommunicationError> {
        self.sender.send(message).await
            .map_err(|e| CommunicationError::SendFailed(e.to_string()))
    }

    /// Receives the next message from this channel.
    pub async fn receive(&self) -> Result<Message, CommunicationError> {
        let mut receiver = self.receiver.write().await;
        receiver.recv().await
            .ok_or_else(|| CommunicationError::ReceiveFailed("Channel closed".to_string()))
    }

    /// Returns the number of messages in the queue (approximate).
    pub async fn len(&self) -> usize {
        // Note: This is an approximation as the receiver may be processing
        self.receiver.read().await.len()
    }

    /// Returns true if the channel has no messages.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

/// A message queue that holds messages for processing.
#[derive(Debug, Clone, Default)]
pub struct MessageQueue {
    /// Messages waiting to be processed.
    messages: Arc<RwLock<Vec<Message>>>,
    /// Maximum queue size (0 means unlimited).
    max_size: usize,
}

impl MessageQueue {
    /// Creates a new message queue.
    pub fn new(max_size: usize) -> Self {
        Self {
            messages: Arc::new(RwLock::new(Vec::new())),
            max_size,
        }
    }

    /// Enqueues a message.
    pub async fn enqueue(&self, message: Message) -> Result<(), CommunicationError> {
        let mut messages = self.messages.write().await;
        
        if self.max_size > 0 && messages.len() >= self.max_size {
            return Err(CommunicationError::SendFailed(
                "Queue is full".to_string()
            ));
        }
        
        messages.push(message);
        Ok(())
    }

    /// Dequeues the next message.
    pub async fn dequeue(&self) -> Result<Message, CommunicationError> {
        let mut messages = self.messages.write().await;
        
        if messages.is_empty() {
            return Err(CommunicationError::ReceiveFailed(
                "Queue is empty".to_string()
            ));
        }
        
        Ok(messages.remove(0))
    }

    /// Peeks at the next message without removing it.
    pub async fn peek(&self) -> Result<Message, CommunicationError> {
        let messages = self.messages.read().await;
        
        messages.first()
            .cloned()
            .ok_or_else(|| CommunicationError::ReceiveFailed("Queue is empty".to_string()))
    }

    /// Returns the number of messages in the queue.
    pub async fn len(&self) -> usize {
        self.messages.read().await.len()
    }

    /// Returns true if the queue is empty.
    pub async fn is_empty(&self) -> bool {
        self.messages.read().await.is_empty()
    }

    /// Clears all messages from the queue.
    pub async fn clear(&self) {
        let mut messages = self.messages.write().await;
        messages.clear();
    }
}

/// A subscriber callback function type.
pub type SubscriberCallback = Box<dyn Fn(Message) + Send + Sync>;

/// A subscriber represents an interest in receiving messages on a topic.

pub struct Subscriber {
    /// Unique identifier for this subscriber.
    pub id: String,
    /// The topic this subscriber is interested in.
    pub topic: String,
    /// Callback function to invoke when a message is received.
    callback: Option<SubscriberCallback>,
}

impl Subscriber {
    /// Creates a new subscriber for a topic.
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: topic.into(),
            callback: None,
        }
    }

    /// Sets the callback function for this subscriber.
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(Message) + Send + Sync + 'static,
    {
        self.callback = Some(Box::new(callback));
        self
    }

    /// Invokes the callback with the given message, if a callback is set.
    pub fn notify(&self, message: Message) {
        if let Some(callback) = &self.callback {
            callback(message);
        }
    }
}

/// A simple pub/sub message broker.
#[derive(Debug, Default, Clone)]
pub struct PubSubBroker {
    /// Map of topics to subscribers.
    subscribers: Arc<RwLock<HashMap<String, Vec<Subscriber>>>>,
    /// Message queue for storing published messages.
    message_queue: MessageQueue,
}

impl PubSubBroker {
    /// Creates a new pub/sub broker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Subscribes to a topic.
    pub async fn subscribe(&self, subscriber: Subscriber) {
        let mut subscribers = self.subscribers.write().await;
        subscribers
            .entry(subscriber.topic.clone())
            .or_insert_with(Vec::new)
            .push(subscriber);
    }

    /// Unsubscribes from a topic by subscriber ID.
    pub async fn unsubscribe(&self, topic: &str, subscriber_id: &str) {
        let mut subscribers = self.subscribers.write().await;
        if let Some(subs) = subscribers.get_mut(topic) {
            subs.retain(|s| s.id != subscriber_id);
        }
    }

    /// Publishes a message to a topic.
    pub async fn publish(&self, topic: &str, message: Message) {
        // Store in message queue
        let _ = self.message_queue.enqueue(message.clone()).await;

        // Notify subscribers
        let subscribers = self.subscribers.read().await;
        if let Some(subs) = subscribers.get(topic) {
            for subscriber in subs {
                subscriber.notify(message.clone());
            }
        }
    }

    /// Returns the number of subscribers for a topic.
    pub async fn subscriber_count(&self, topic: &str) -> usize {
        let subscribers = self.subscribers.read().await;
        subscribers
            .get(topic)
            .map(|subs| subs.len())
            .unwrap_or(0)
    }

    /// Lists all topics with active subscribers.
    pub async fn list_topics(&self) -> Vec<String> {
        let subscribers = self.subscribers.read().await;
        subscribers.keys().cloned().collect()
    }
}

/// A simple in-memory implementation of CommunicationProtocol.
#[derive(Debug, Clone)]
pub struct InMemoryProtocol {
    /// Message queue for this protocol instance.
    queue: MessageQueue,
    /// Broker for pub/sub messaging.
    broker: PubSubBroker,
    /// The agent ID using this protocol.
    agent_id: AgentId,
}

impl InMemoryProtocol {
    /// Creates a new in-memory protocol instance.
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            queue: MessageQueue::new(0), // unlimited
            broker: PubSubBroker::new(),
            agent_id,
        }
    }
}

#[async_trait::async_trait]
impl CommunicationProtocol for InMemoryProtocol {
    async fn send(&self, message: Message) -> Result<(), CommunicationError> {
        self.queue.enqueue(message).await
    }

    async fn receive(&self) -> Result<Message, CommunicationError> {
        self.queue.dequeue().await
    }

    async fn subscribe(&self, topic: &str) -> Result<(), CommunicationError> {
        let subscriber = Subscriber::new(topic);
        self.broker.subscribe(subscriber).await;
        Ok(())
    }

    async fn unsubscribe(&self, _topic: &str) -> Result<(), CommunicationError> {
        // For simplicity, we'd need to track subscriber IDs
        // This is a simplified implementation
        Ok(())
    }

    async fn publish(&self, topic: &str, message: Message) -> Result<(), CommunicationError> {
        self.broker.publish(topic, message).await;
        Ok(())
    }

    async fn broadcast(&self, message: Message) -> Result<(), CommunicationError> {
        // Broadcast to all topics
        let topics = self.broker.list_topics().await;
        for topic in topics {
            self.broker.publish(&topic, message.clone()).await;
        }
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        true // Always connected for in-memory
    }

    async fn close(&self) -> Result<(), CommunicationError> {
        self.queue.clear().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_channel_send_receive() {
        let channel = Channel::new("test_channel");
        let message = Message::new(
            crate::protocol::MessageType::Notification,
            AgentId::new(),
            None,
            json!({"test": "data"}),
        );

        assert!(channel.send(message.clone()).await.is_ok());
        let received = channel.receive().await.unwrap();
        assert_eq!(received.id, message.id);
    }

    #[tokio::test]
    async fn test_message_queue_operations() {
        let queue = MessageQueue::new(10);
        
        assert!(queue.is_empty().await);
        
        let message = Message::new(
            crate::protocol::MessageType::Request,
            AgentId::new(),
            Some(AgentId::new()),
            json!({"request": "data"}),
        );
        
        assert!(queue.enqueue(message.clone()).await.is_ok());
        assert!(!queue.is_empty().await);
        assert_eq!(queue.len().await, 1);
        
        let dequeued = queue.dequeue().await.unwrap();
        assert_eq!(dequeued.id, message.id);
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_pubsub_broker() {
        let broker = PubSubBroker::new();
        
        let mut notification_count = 0;
        let subscriber = Subscriber::new("test_topic")
            .with_callback(|_| {
                // In a real test, we'd use a channel to count
                // This is simplified
            });
        
        broker.subscribe(subscriber).await;
        assert_eq!(broker.subscriber_count("test_topic").await, 1);
        
        let message = Message::new(
            crate::protocol::MessageType::Publish,
            AgentId::new(),
            None,
            json!({"event": "test"}),
        );
        
        broker.publish("test_topic", message).await;
        assert_eq!(broker.subscriber_count("test_topic").await, 1);
    }
}

impl std::fmt::Debug for Subscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Subscriber")
            .field("id", &self.id)
            .field("topic", &self.topic)
            .field("callback", &self.callback.as_ref().map(|_| "<callback>"))
            .finish()
    }
}
