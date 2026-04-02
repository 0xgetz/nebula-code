//! Agent registry for tracking available agents and their capabilities.

use crate::types::{Agent, AgentCapability, AgentId, AgentState};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

/// Errors that can occur in registry operations.
#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Agent already registered: {0}")]
    AgentAlreadyRegistered(String),
    #[error("No agent found with capability: {0}")]
    NoAgentWithCapability(String),
    #[error("Registry is locked")]
    RegistryLocked,
}

/// Registry for tracking available agents in the system.
#[derive(Debug, Default)]
pub struct AgentRegistry {
    /// Map of agent IDs to agents.
    agents: Arc<RwLock<HashMap<AgentId, Agent>>>,
    /// Map of capability names to agent IDs that provide them.
    capabilities_index: Arc<RwLock<HashMap<String, Vec<AgentId>>>>,
}

impl AgentRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an agent in the registry.
    pub fn register(&self, agent: Agent) -> Result<(), RegistryError> {
        let mut agents = self.agents
            .write()
            .map_err(|_| RegistryError::RegistryLocked)?;
        
        if agents.contains_key(&agent.id) {
            return Err(RegistryError::AgentAlreadyRegistered(
                agent.id.to_string()
            ));
        }

        // Index capabilities
        let mut cap_index = self.capabilities_index
            .write()
            .map_err(|_| RegistryError::RegistryLocked)?;
        
        for capability in &agent.capabilities {
            cap_index
                .entry(capability.name.clone())
                .or_insert_with(Vec::new)
                .push(agent.id.clone());
        }

        agents.insert(agent.id.clone(), agent);
        Ok(())
    }

    /// Unregisters an agent from the registry.
    pub fn unregister(&self, agent_id: &AgentId) -> Result<Agent, RegistryError> {
        let mut agents = self.agents
            .write()
            .map_err(|_| RegistryError::RegistryLocked)?;
        
        let agent = agents.remove(agent_id)
            .ok_or_else(|| RegistryError::AgentNotFound(agent_id.to_string()))?;

        // Remove from capability index
        let mut cap_index = self.capabilities_index
            .write()
            .map_err(|_| RegistryError::RegistryLocked)?;
        
        for capability in &agent.capabilities {
            if let Some(agent_ids) = cap_index.get_mut(&capability.name) {
                agent_ids.retain(|id| id != agent_id);
                if agent_ids.is_empty() {
                    cap_index.remove(&capability.name);
                }
            }
        }

        Ok(agent)
    }

    /// Gets an agent by ID.
    pub fn get_agent(&self, agent_id: &AgentId) -> Result<Agent, RegistryError> {
        let agents = self.agents
            .read()
            .map_err(|_| RegistryError::RegistryLocked)?;
        
        agents.get(agent_id)
            .cloned()
            .ok_or_else(|| RegistryError::AgentNotFound(agent_id.to_string()))
    }

    /// Updates an agent's state.
    pub fn update_state(&self, agent_id: &AgentId, state: AgentState) -> Result<(), RegistryError> {
        let mut agents = self.agents
            .write()
            .map_err(|_| RegistryError::RegistryLocked)?;
        
        let agent = agents.get_mut(agent_id)
            .ok_or_else(|| RegistryError::AgentNotFound(agent_id.to_string()))?;
        
        agent.set_state(state);
        Ok(())
    }

    /// Finds agents that have a specific capability.
    pub fn find_by_capability(&self, capability: &str) -> Result<Vec<Agent>, RegistryError> {
        let cap_index = self.capabilities_index
            .read()
            .map_err(|_| RegistryError::RegistryLocked)?;
        let agents = self.agents
            .read()
            .map_err(|_| RegistryError::RegistryLocked)?;

        let agent_ids = cap_index.get(capability)
            .ok_or_else(|| RegistryError::NoAgentWithCapability(capability.to_string()))?;

        Ok(agent_ids
            .iter()
            .filter_map(|id| agents.get(id).cloned())
            .collect())
    }

    /// Lists all registered agents.
    pub fn list_agents(&self) -> Result<Vec<Agent>, RegistryError> {
        let agents = self.agents
            .read()
            .map_err(|_| RegistryError::RegistryLocked)?;
        Ok(agents.values().cloned().collect())
    }

    /// Returns the number of registered agents.
    pub fn count(&self) -> Result<usize, RegistryError> {
        let agents = self.agents
            .read()
            .map_err(|_| RegistryError::RegistryLocked)?;
        Ok(agents.len())
    }

    /// Checks if an agent is registered.
    pub fn contains(&self, agent_id: &AgentId) -> Result<bool, RegistryError> {
        let agents = self.agents
            .read()
            .map_err(|_| RegistryError::RegistryLocked)?;
        Ok(agents.contains_key(agent_id))
    }

    /// Gets all available capabilities across all agents.
    pub fn list_capabilities(&self) -> Result<Vec<AgentCapability>, RegistryError> {
        let agents = self.agents
            .read()
            .map_err(|_| RegistryError::RegistryLocked)?;
        
        Ok(agents
            .values()
            .flat_map(|agent| agent.capabilities.clone())
            .collect())
    }

    /// Gets agents by their current state.
    pub fn find_by_state(&self, state: AgentState) -> Result<Vec<Agent>, RegistryError> {
        let agents = self.agents
            .read()
            .map_err(|_| RegistryError::RegistryLocked)?;
        
        Ok(agents
            .values()
            .filter(|agent| agent.state == state)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentMetadata;

    fn create_test_agent(name: &str) -> Agent {
        let id = AgentId::new();
        let mut agent = Agent::new(id, AgentMetadata::new().with_name(name));
        agent.add_capability(AgentCapability::new("test_cap", "Test capability", "1.0"));
        agent
    }

    #[test]
    fn test_register_and_get_agent() {
        let registry = AgentRegistry::new();
        let agent = create_test_agent("TestAgent");
        let id = agent.id.clone();

        assert!(registry.register(agent).is_ok());
        let retrieved = registry.get_agent(&id).unwrap();
        assert_eq!(retrieved.metadata.name.unwrap(), "TestAgent");
    }

    #[test]
    fn test_unregister_agent() {
        let registry = AgentRegistry::new();
        let agent = create_test_agent("TestAgent");
        let id = agent.id.clone();

        assert!(registry.register(agent).is_ok());
        let removed = registry.unregister(&id).unwrap();
        assert_eq!(removed.metadata.name.unwrap(), "TestAgent");
        assert!(registry.get_agent(&id).is_err());
    }

    #[test]
    fn test_find_by_capability() {
        let registry = AgentRegistry::new();
        let agent1 = create_test_agent("Agent1");
        let agent2 = create_test_agent("Agent2");

        assert!(registry.register(agent1).is_ok());
        assert!(registry.register(agent2).is_ok());

        let agents = registry.find_by_capability("test_cap").unwrap();
        assert_eq!(agents.len(), 2);
    }

    #[test]
    fn test_list_agents() {
        let registry = AgentRegistry::new();
        assert!(registry.register(create_test_agent("Agent1")).is_ok());
        assert!(registry.register(create_test_agent("Agent2")).is_ok());

        let agents = registry.list_agents().unwrap();
        assert_eq!(agents.len(), 2);
    }
}
