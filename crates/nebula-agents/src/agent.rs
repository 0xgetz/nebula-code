use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during agent execution
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("LLM request failed: {0}")]
    LlmError(String),
    
    #[error("Skill execution failed: {0}")]
    SkillError(String),
    
    #[error("File operation failed: {0}")]
    FileError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Agent timeout: {0}")]
    TimeoutError(String),
}

pub type AgentResult<T> = Result<T, AgentError>;

/// Context passed to agents during execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    /// Current project directory
    pub project_dir: String,
    
    /// Available skill cards
    pub skills: Vec<String>,
    
    /// Current model configuration
    pub model: String,
    
    /// User preferences
    pub preferences: AgentPreferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreferences {
    pub code_style: String,
    pub test_coverage: f32,
    pub security_level: SecurityLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityLevel {
    Low,
    Medium,
    High,
}

impl Default for AgentPreferences {
    fn default() -> Self {
        Self {
            code_style: "rustfmt".to_string(),
            test_coverage: 0.8,
            security_level: SecurityLevel::High,
        }
    }
}

/// Base trait for all agents
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get the agent type
    fn agent_type(&self) -> crate::AgentType;
    
    /// Get a description of what this agent does
    fn description(&self) -> &'static str;
    
    /// Execute the agent's main task
    async fn execute(&self, input: &str, context: &AgentContext) -> AgentResult<AgentOutput>;
    
    /// Validate if this agent can handle the given task
    fn can_handle(&self, task: &str) -> bool;
}

/// Output from agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    /// Generated content (code, plan, review, etc.)
    pub content: String,
    
    /// Files created or modified
    pub files: Vec<AgentFile>,
    
    /// Agent's reasoning or explanation
    pub reasoning: String,
    
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
}

/// File created or modified by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentFile {
    /// File path relative to project root
    pub path: String,
    
    /// File content
    pub content: String,
    
    /// Whether this is a new file or modification
    pub action: FileAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileAction {
    Create,
    Modify,
    Delete,
}
