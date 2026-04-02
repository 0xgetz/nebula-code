use crate::dependencies::Dependency;
use crate::rating::SkillRating;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur in the marketplace
#[derive(Error, Debug)]
pub enum MarketplaceError {
    #[error("Skill not found: {0}")]
    SkillNotFound(String),
    #[error("Invalid skill version: {0}")]
    InvalidVersion(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for marketplace operations
pub type Result<T> = std::result::Result<T, MarketplaceError>;

/// Categories for skills
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SkillCategory {
    CodeGeneration,
    DataAnalysis,
    Automation,
    Communication,
    Research,
    Development,
    DevOps,
    Security,
    Monitoring,
    Testing,
    Documentation,
    Other,
}

impl std::fmt::Display for SkillCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillCategory::CodeGeneration => write!(f, "Code Generation"),
            SkillCategory::DataAnalysis => write!(f, "Data Analysis"),
            SkillCategory::Automation => write!(f, "Automation"),
            SkillCategory::Communication => write!(f, "Communication"),
            SkillCategory::Research => write!(f, "Research"),
            SkillCategory::Development => write!(f, "Development"),
            SkillCategory::DevOps => write!(f, "DevOps"),
            SkillCategory::Security => write!(f, "Security"),
            SkillCategory::Monitoring => write!(f, "Monitoring"),
            SkillCategory::Testing => write!(f, "Testing"),
            SkillCategory::Documentation => write!(f, "Documentation"),
            SkillCategory::Other => write!(f, "Other"),
        }
    }
}

/// Version information for a skill
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
}

impl SkillVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            prerelease: None,
        }
    }

    pub fn with_prerelease(mut self, prerelease: String) -> Self {
        self.prerelease = Some(prerelease);
        self
    }
}

impl std::fmt::Display for SkillVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.prerelease {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

/// Metadata about a skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub author: String,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub categories: Vec<SkillCategory>,
    pub tags: Vec<String>,
    pub version: SkillVersion,
    pub dependencies: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl SkillMetadata {
    pub fn new(name: String, description: String, author: String) -> Self {
        Self {
            name,
            description,
            author,
            license: None,
            homepage: None,
            repository: None,
            categories: Vec::new(),
            tags: Vec::new(),
            version: SkillVersion::new(0, 1, 0),
            dependencies: Vec::new(),
            created_at: None,
            updated_at: None,
        }
    }

    pub fn with_categories(mut self, categories: Vec<SkillCategory>) -> Self {
        self.categories = categories;
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_version(mut self, version: SkillVersion) -> Self {
        self.version = version;
        self
    }
}

/// A skill in the marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub metadata: SkillMetadata,
    pub manifest: SkillManifest,
    pub installed: bool,
    pub install_path: Option<String>,
    pub rating: Option<SkillRating>,
    pub dependencies: Vec<Dependency>,
}

impl Skill {
    pub fn new(id: String, metadata: SkillMetadata, manifest: SkillManifest) -> Self {
        Self {
            id,
            metadata,
            manifest,
            installed: false,
            install_path: None,
            rating: None,
            dependencies: Vec::new(),
        }
    }
    /// Adds a dependency to this skill.
    pub fn with_dependencies(mut self, dependencies: Vec<Dependency>) -> Self {
        self.dependencies = dependencies;
        self
    }

}

/// Manifest describing the skill's entry points and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub entry_point: String,
    pub language: String,
    pub runtime: Option<String>,
    pub config_schema: Option<serde_json::Value>,
    pub environment: HashMap<String, String>,
    pub permissions: Vec<String>,
}

impl SkillManifest {
    pub fn new(entry_point: String, language: String) -> Self {
        Self {
            entry_point,
            language,
            runtime: None,
            config_schema: None,
            environment: HashMap::new(),
            permissions: Vec::new(),
        }
    }

    pub fn with_runtime(mut self, runtime: String) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_version_display() {
        let version = SkillVersion::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");

        let version_with_pre = version.clone().with_prerelease("alpha".to_string());
        assert_eq!(version_with_pre.to_string(), "1.2.3-alpha");
    }

    #[test]
    fn test_skill_metadata_creation() {
        let metadata = SkillMetadata::new(
            "test-skill".to_string(),
            "A test skill".to_string(),
            "author".to_string(),
        );
        assert_eq!(metadata.name, "test-skill");
        assert_eq!(metadata.version.to_string(), "0.1.0");
    }

    #[test]
    fn test_skill_category_display() {
        assert_eq!(SkillCategory::CodeGeneration.to_string(), "Code Generation");
        assert_eq!(SkillCategory::DataAnalysis.to_string(), "Data Analysis");
    }
}
