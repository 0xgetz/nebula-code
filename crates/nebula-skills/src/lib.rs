//! Nebula Skills - Skill card system for reusable coding workflows
//!
//! This crate provides the data models and management system for skill cards,
//! which are reusable coding patterns and workflows that can be shared and sold.

use serde::{Serialize, Deserialize};
mod skill;
mod store;
mod marketplace;

pub use skill::{SkillCard, SkillMetadata, SkillFile, SkillCategory, FileAction};
pub use store::{SkillStore, SkillStoreError};
pub use marketplace::{Marketplace, MarketplaceError};

/// Skill version format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for SkillVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::str::FromStr for SkillVersion {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err("Invalid version format".to_string());
        }
        
        Ok(Self {
            major: parts[0].parse().map_err(|_| "Invalid major version")?,
            minor: parts[1].parse().map_err(|_| "Invalid minor version")?,
            patch: parts[2].parse().map_err(|_| "Invalid patch version")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_display() {
        let version = SkillVersion { major: 1, minor: 2, patch: 3 };
        assert_eq!(version.to_string(), "1.2.3");
    }
    
    #[test]
    fn test_version_parsing() {
        let version: SkillVersion = "1.2.3".parse().unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }
}
