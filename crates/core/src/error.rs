use thiserror::Error;

/// Error types for Nebula Core
#[derive(Error, Debug)]
pub enum NebulaError {
    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Skill card not found: {0}")]
    SkillNotFound(String),

    #[error("Invalid skill card: {0}")]
    InvalidSkillCard(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Build error: {0}")]
    BuildError(String),

    #[error("Deployment error: {0}")]
    DeployError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),
}

pub type Result<T> = std::result::Result<T, NebulaError>;
