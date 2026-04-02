//! Core agent types and communication protocol for Nebula.
//!
//! This crate provides the foundational types and traits for building
//! multi-agent systems within the Nebula framework.

pub mod communication;
pub mod protocol;
pub mod registry;
pub mod types;
pub mod orchestration;

pub use communication::{Channel, MessageQueue, Subscriber};
pub use protocol::{CommunicationProtocol, Message, MessageType};
pub use registry::AgentRegistry;
pub use types::{Agent, AgentCapability, AgentId, AgentMetadata, AgentState};
pub use orchestration::{
    Orchestrator, Task, TaskId, TaskScheduler, TaskStatus, Priority, OrchestrationError,
};
