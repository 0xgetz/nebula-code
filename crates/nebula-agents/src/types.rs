//! Core agent types and data structures.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(Uuid);

impl AgentId {
    /// Creates a new random agent ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates an agent ID from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the current state of an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// Agent is idle and waiting for tasks.
    Idle,
    /// Agent is currently processing a task.
    Busy,
    /// Agent is offline or unreachable.
    Offline,
    /// Agent encountered an error.
    Error,
    /// Agent is starting up.
    Initializing,
}

impl Default for AgentState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Represents a capability that an agent possesses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCapability {
    /// Unique name for this capability.
    pub name: String,
    /// Description of what this capability does.
    pub description: String,
    /// Version of this capability.
    pub version: String,
    /// Optional metadata about the capability.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentCapability {
    pub fn new(name: impl Into<String>, description: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            version: version.into(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Metadata associated with an agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMetadata {
    /// Human-readable name of the agent.
    #[serde(default)]
    pub name: Option<String>,
    /// Description of the agent's purpose.
    #[serde(default)]
    pub description: Option<String>,
    /// Owner or creator of the agent.
    #[serde(default)]
    pub owner: Option<String>,
    /// Additional key-value metadata.
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

impl AgentMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(owner.into());
        self
    }

    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }
}

/// Represents an agent in the Nebula system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique identifier for this agent.
    pub id: AgentId,
    /// Current state of the agent.
    pub state: AgentState,
    /// Capabilities this agent provides.
    pub capabilities: Vec<AgentCapability>,
    /// Metadata about the agent.
    pub metadata: AgentMetadata,
    /// Timestamp when the agent was created.
    #[serde(with = "chrono_serializer")]
    pub created_at: std::time::SystemTime,
    /// Timestamp of the last state change.
    #[serde(with = "chrono_serializer")]
    pub last_updated: std::time::SystemTime,
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

impl Agent {
    /// Creates a new agent with the given ID and metadata.
    pub fn new(id: AgentId, metadata: AgentMetadata) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            id,
            state: AgentState::Initializing,
            capabilities: Vec::new(),
            metadata,
            created_at: now,
            last_updated: now,
        }
    }

    /// Adds a capability to the agent.
    pub fn add_capability(&mut self, capability: AgentCapability) {
        self.capabilities.push(capability);
        self.last_updated = std::time::SystemTime::now();
    }

    /// Sets the agent's state.
    pub fn set_state(&mut self, state: AgentState) {
        self.state = state;
        self.last_updated = std::time::SystemTime::now();
    }

    /// Returns true if the agent has a capability with the given name.
    pub fn has_capability(&self, name: &str) -> bool {
        self.capabilities.iter().any(|c| c.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_creation() {
        let id = AgentId::new();
        assert_ne!(id.to_string(), "");
    }

    #[test]
    fn test_agent_creation() {
        let id = AgentId::new();
        let metadata = AgentMetadata::new().with_name("TestAgent");
        let agent = Agent::new(id, metadata);
        assert_eq!(agent.state, AgentState::Initializing);
        assert!(agent.capabilities.is_empty());
    }

    #[test]
    fn test_add_capability() {
        let id = AgentId::new();
        let mut agent = Agent::new(id, AgentMetadata::new());
        agent.add_capability(AgentCapability::new("test", "A test capability", "1.0"));
        assert!(agent.has_capability("test"));
        assert!(!agent.has_capability("nonexistent"));
    }
}
