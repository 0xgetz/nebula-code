//! Nebula Agents - Multi-agent orchestration system
//!
//! This crate provides the core agent framework for Nebula Code,
//! including specialized agents for different aspects of software development.

mod agent;
mod architect;
mod coder;
mod tester;
mod reviewer;
mod deployer;
mod orchestrator;

pub use agent::{Agent, AgentError, AgentResult};
pub use architect::ArchitectAgent;
pub use coder::CoderAgent;
pub use tester::TesterAgent;
pub use reviewer::ReviewerAgent;
pub use deployer::DeployerAgent;
pub use orchestrator::AgentOrchestrator;

/// Agent types available in Nebula Code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    Architect,
    Coder,
    Tester,
    Reviewer,
    Deployer,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::Architect => write!(f, "Architect"),
            AgentType::Coder => write!(f, "Coder"),
            AgentType::Tester => write!(f, "Tester"),
            AgentType::Reviewer => write!(f, "Reviewer"),
            AgentType::Deployer => write!(f, "Deployer"),
        }
    }
}
