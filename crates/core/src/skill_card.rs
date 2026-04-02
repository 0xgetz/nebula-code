use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a skill card in the Nebula ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCard {
    /// Unique identifier for the skill card
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Description of what the skill does
    pub description: String,
    
    /// Version following semver
    pub version: String,
    
    /// Author information
    pub author: SkillAuthor,
    
    /// Category of the skill
    pub category: SkillCategory,
    
    /// Tags for discovery
    pub tags: Vec<String>,
    
    /// Configuration schema as JSON
    pub config_schema: Option<serde_json::Value>,
    
    /// Default configuration values
    pub default_config: HashMap<String, serde_json::Value>,
    
    /// Dependencies on other skill cards
    pub dependencies: Vec<SkillDependency>,
    
    /// License identifier (e.g., "MIT", "Apache-2.0")
    pub license: String,
    
    /// Repository URL
    pub repository: Option<String>,
    
    /// Documentation URL
    pub documentation: Option<String>,
}

/// Author information for a skill card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAuthor {
    /// Author name
    pub name: String,
    
    /// Author email
    pub email: Option<String>,
    
    /// Author URL
    pub url: Option<String>,
}

/// Category of a skill card
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SkillCategory {
    CodeGeneration,
    CodeReview,
    Testing,
    Documentation,
    Deployment,
    Performance,
    Security,
    DataProcessing,
    Utilities,
    Other,
}

/// Dependency on another skill card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    /// ID of the required skill
    pub skill_id: String,
    
    /// Version requirement (semver range)
    pub version: String,
}

impl SkillCard {
    /// Create a new skill card with minimal required fields
    pub fn new(id: impl Into<String>, name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            version: "0.1.0".to_string(),
            author: SkillAuthor {
                name: "Unknown".to_string(),
                email: None,
                url: None,
            },
            category: SkillCategory::Other,
            tags: vec![],
            config_schema: None,
            default_config: HashMap::new(),
            dependencies: vec![],
            license: "MIT".to_string(),
            repository: None,
            documentation: None,
        }
    }
    
    /// Validate the skill card structure
    pub fn validate(&self) -> Result<(), String> {
        if self.id.is_empty() {
            return Err("Skill card ID cannot be empty".to_string());
        }
        if self.name.is_empty() {
            return Err("Skill card name cannot be empty".to_string());
        }
        if self.description.is_empty() {
            return Err("Skill card description cannot be empty".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_card_creation() {
        let card = SkillCard::new("test-skill", "Test Skill", "A test skill card");
        assert_eq!(card.id, "test-skill");
        assert_eq!(card.name, "Test Skill");
        assert_eq!(card.description, "A test skill card");
    }

    #[test]
    fn test_skill_card_validation() {
        let valid_card = SkillCard::new("valid", "Valid", "Valid description");
        assert!(valid_card.validate().is_ok());

        let invalid_card = SkillCard::new("", "Invalid", "Invalid description");
        assert!(invalid_card.validate().is_err());
    }
}
