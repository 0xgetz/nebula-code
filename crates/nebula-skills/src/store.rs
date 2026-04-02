use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;
use crate::skill::SkillCard;

/// Errors that can occur in skill store operations
#[derive(Error, Debug)]
pub enum SkillStoreError {
    #[error("Skill not found: {0}")]
    NotFound(String),
    
    #[error("File operation failed: {0}")]
    FileError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Invalid skill data: {0}")]
    InvalidData(String),
}

pub type SkillStoreResult<T> = Result<T, SkillStoreError>;

/// Local skill store - manages installed skills
pub struct SkillStore {
    skills: HashMap<String, SkillCard>,
    store_dir: String,
}

impl SkillStore {
    /// Create a new skill store
    pub fn new(store_dir: &str) -> Self {
        Self {
            skills: HashMap::new(),
            store_dir: store_dir.to_string(),
        }
    }
    
    /// Load skills from disk
    pub fn load(&mut self) -> SkillStoreResult<()> {
        // TODO: Implement loading from disk
        Ok(())
    }
    
    /// Save skills to disk
    pub fn save(&self) -> SkillStoreResult<()> {
        // TODO: Implement saving to disk
        Ok(())
    }
    
    /// Add a skill to the store
    pub fn add(&mut self, skill: SkillCard) -> SkillStoreResult<()> {
        self.skills.insert(skill.id.clone(), skill);
        Ok(())
    }
    
    /// Remove a skill from the store
    pub fn remove(&mut self, skill_id: &str) -> SkillStoreResult<()> {
        self.skills.remove(skill_id)
            .ok_or_else(|| SkillStoreError::NotFound(skill_id.to_string()))?;
        Ok(())
    }
    
    /// Get a skill by ID
    pub fn get(&self, skill_id: &str) -> SkillStoreResult<&SkillCard> {
        self.skills.get(skill_id)
            .ok_or_else(|| SkillStoreError::NotFound(skill_id.to_string()))
    }
    
    /// List all skills
    pub fn list(&self) -> Vec<&SkillCard> {
        self.skills.values().collect()
    }
    
    /// Search skills by query
    pub fn search(&self, query: &str) -> Vec<&SkillCard> {
        self.skills.values()
            .filter(|skill| {
                skill.name.to_lowercase().contains(&query.to_lowercase()) ||
                skill.description.to_lowercase().contains(&query.to_lowercase()) ||
                skill.tags.iter().any(|tag| tag.to_lowercase().contains(&query.to_lowercase()))
            })
            .collect()
    }
    
    /// Get skills by category
    pub fn by_category(&self, category: crate::SkillCategory) -> Vec<&SkillCard> {
        self.skills.values()
            .filter(|skill| skill.category == category)
            .collect()
    }
}

impl Default for SkillStore {
    fn default() -> Self {
        Self::new("~/.nebula/skills")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::SkillCard;
    
    #[test]
    fn test_skill_store_operations() {
        let mut store = SkillStore::new("/tmp/test");
        
        let skill = SkillCard::new(
            "Test Skill".to_string(),
            "A test skill".to_string(),
            "tester".to_string(),
        );
        
        store.add(skill.clone()).unwrap();
        
        let retrieved = store.get(&skill.id).unwrap();
        assert_eq!(retrieved.name, "Test Skill");
        
        let skills = store.list();
        assert_eq!(skills.len(), 1);
        
        store.remove(&skill.id).unwrap();
        assert!(store.get(&skill.id).is_err());
    }
}
