use serde::{Deserialize, Serialize};
use crate::SkillVersion;

/// Skill card - a reusable coding pattern or workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCard {
    /// Unique identifier
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Detailed description
    pub description: String,
    
    /// Version following semver
    pub version: SkillVersion,
    
    /// Author/creator
    pub author: String,
    
    /// Price in USD (0 for free)
    pub price: f32,
    
    /// Category
    pub category: SkillCategory,
    
    /// Tags for searchability
    pub tags: Vec<String>,
    
    /// Compatibility with other tools
    pub compatibility: Vec<String>,
    
    /// Files included in the skill
    pub files: Vec<SkillFile>,
    
    /// Dependencies required
    pub dependencies: Vec<String>,
    
    /// User rating (0.0 to 5.0)
    pub rating: f32,
    
    /// Number of downloads
    pub downloads: u32,
    
    /// Metadata
    pub metadata: SkillMetadata,
}

/// Skill categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCategory {
    Auth,
    Database,
    Api,
    Testing,
    Deployment,
    Security,
    Performance,
    UiComponent,
    StateManagement,
    Utilities,
    Other,
}

impl std::fmt::Display for SkillCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillCategory::Auth => write!(f, "Authentication"),
            SkillCategory::Database => write!(f, "Database"),
            SkillCategory::Api => write!(f, "API"),
            SkillCategory::Testing => write!(f, "Testing"),
            SkillCategory::Deployment => write!(f, "Deployment"),
            SkillCategory::Security => write!(f, "Security"),
            SkillCategory::Performance => write!(f, "Performance"),
            SkillCategory::UiComponent => write!(f, "UI Component"),
            SkillCategory::StateManagement => write!(f, "State Management"),
            SkillCategory::Utilities => write!(f, "Utilities"),
            SkillCategory::Other => write!(f, "Other"),
        }
    }
}

/// Individual file in a skill card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFile {
    /// File path relative to skill root
    pub path: String,
    
    /// File content
    pub content: String,
    
    /// File type/extension
    pub file_type: String,
    
    /// Whether to overwrite existing files
    pub overwrite: bool,
}

/// Additional metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillMetadata {
    /// Creation timestamp
    pub created_at: String,
    
    /// Last update timestamp
    pub updated_at: String,
    
    /// Minimum Nebula Code version required
    pub min_nebula_version: Option<String>,
    
    /// License
    pub license: Option<String>,
    
    /// Documentation URL
    pub documentation_url: Option<String>,
    
    /// Support URL
    pub support_url: Option<String>,
}

/// File action for skill installation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileAction {
    Create,
    Modify,
    Delete,
}

impl SkillCard {
    /// Create a new skill card
    pub fn new(name: String, description: String, author: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(), // TODO: Use proper ID generation
            name,
            description,
            version: SkillVersion { major: 1, minor: 0, patch: 0 },
            author,
            price: 0.0,
            category: SkillCategory::Other,
            tags: vec![],
            compatibility: vec![],
            files: vec![],
            dependencies: vec![],
            rating: 0.0,
            downloads: 0,
            metadata: SkillMetadata::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_skill_card_creation() {
        let skill = SkillCard::new(
            "Next.js Auth".to_string(),
            "Authentication with Next.js".to_string(),
            "0xgetz".to_string(),
        );
        assert_eq!(skill.name, "Next.js Auth");
        assert_eq!(skill.version.major, 1);
    }
}
