use crate::discovery::{SkillDiscovery, SkillFilter, SkillQuery};
use crate::rating::RatingQuery;
use crate::types::{MarketplaceError, Result, Skill, SkillCategory, SkillVersion};
use std::collections::HashMap;

/// Registry for storing and managing skills
#[derive(Debug, Default)]
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a skill in the registry
    pub fn register(&mut self, skill: Skill) -> Result<()> {
        if self.skills.contains_key(&skill.id) {
            return Err(MarketplaceError::InvalidVersion(format!(
                "Skill with id {} already exists",
                skill.id
            )));
        }
        self.skills.insert(skill.id.clone(), skill);
        Ok(())
    }

    /// Get a skill by its ID
    pub fn get(&self, id: &str) -> Option<&Skill> {
        self.skills.get(id)
    }

    /// Get a mutable reference to a skill by its ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Skill> {
        self.skills.get_mut(id)
    }

    /// Remove a skill from the registry
    pub fn unregister(&mut self, id: &str) -> Result<Skill> {
        self.skills
            .remove(id)
            .ok_or_else(|| MarketplaceError::SkillNotFound(id.to_string()))
    }

    /// Check if a skill exists
    pub fn contains(&self, id: &str) -> bool {
        self.skills.contains_key(id)
    }

    /// Get all skills in the registry
    pub fn list_all(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }

    /// Get the number of skills in the registry
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Find skills by category
    pub fn find_by_category(&self, category: &SkillCategory) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|skill| skill.metadata.categories.contains(category))
            .collect()
    }

    /// Find skills by version
    pub fn find_by_version(&self, version: &SkillVersion) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|skill| &skill.metadata.version == version)
            .collect()
    }

    /// Find skills by name (partial match)
    pub fn find_by_name(&self, name: &str) -> Vec<&Skill> {
        let query = name.to_lowercase();
        self.skills
            .values()
            .filter(|skill| skill.metadata.name.to_lowercase().contains(&query))
            .collect()
    }

    /// Find skills by tag
    pub fn find_by_tag(&self, tag: &str) -> Vec<&Skill> {
        let query = tag.to_lowercase();
        self.skills
            .values()
            .filter(|skill| skill.metadata.tags.iter().any(|t| t.to_lowercase() == query))
            .collect()
    }

    /// Find installed skills
    pub fn find_installed(&self) -> Vec<&Skill> {
        self.skills.values().filter(|skill| skill.installed).collect()
    }

    /// Find not installed skills
    pub fn find_not_installed(&self) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|skill| !skill.installed)
            .collect()
    }

    /// Mark a skill as installed
    pub fn mark_installed(&mut self, id: &str, path: Option<String>) -> Result<()> {
        let skill = self
            .skills
            .get_mut(id)
            .ok_or_else(|| MarketplaceError::SkillNotFound(id.to_string()))?;
        skill.installed = true;
        skill.install_path = path;
        Ok(())
    }

    /// Mark a skill as not installed
    pub fn mark_uninstalled(&mut self, id: &str) -> Result<()> {
        let skill = self
            .skills
            .get_mut(id)
            .ok_or_else(|| MarketplaceError::SkillNotFound(id.to_string()))?;
        skill.installed = false;
        skill.install_path = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SkillManifest, SkillMetadata};

    fn create_test_skill(id: &str, name: &str) -> Skill {
        let metadata = SkillMetadata::new(name.to_string(), "desc".to_string(), "author".to_string());
        let manifest = SkillManifest::new("main.rs".to_string(), "rust".to_string());
        Skill::new(id.to_string(), metadata, manifest)
    }

    #[test]
    fn test_registry_creation() {
        let registry = SkillRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_register_and_get() {
        let mut registry = SkillRegistry::new();
        let skill = create_test_skill("test-1", "Test Skill");
        assert!(registry.register(skill).is_ok());
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.get("test-1").is_some());
        assert!(registry.get("non-existent").is_none());
    }

    #[test]
    fn test_unregister() {
        let mut registry = SkillRegistry::new();
        let skill = create_test_skill("test-1", "Test Skill");
        registry.register(skill).unwrap();
        assert!(registry.unregister("test-1").is_ok());
        assert!(registry.get("test-1").is_none());
        assert!(registry.unregister("test-1").is_err());
    }

    #[test]
    fn test_find_by_name() {
        let mut registry = SkillRegistry::new();
        registry.register(create_test_skill("1", "Alpha Skill")).unwrap();
        registry.register(create_test_skill("2", "Beta Skill")).unwrap();
        registry.register(create_test_skill("3", "Gamma Skill")).unwrap();

        let results = registry.find_by_name("alpha");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.name, "Alpha Skill");

        let results = registry.find_by_name("skill");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_find_by_category() {
        let mut registry = SkillRegistry::new();
        let mut skill1 = create_test_skill("1", "Code Skill");
        skill1.metadata.categories.push(SkillCategory::CodeGeneration);
        let skill2 = create_test_skill("2", "Data Skill");

        registry.register(skill1).unwrap();
        registry.register(skill2).unwrap();

        let results = registry.find_by_category(&SkillCategory::CodeGeneration);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.name, "Code Skill");
    }

    #[test]
    fn test_install_operations() {
        let mut registry = SkillRegistry::new();
        registry.register(create_test_skill("1", "Test")).unwrap();

        assert!(registry.mark_installed("1", Some("/path/to/skill".to_string())).is_ok());
        assert!(registry.get("1").unwrap().installed);
        assert_eq!(
            registry.get("1").unwrap().install_path,
            Some("/path/to/skill".to_string())
        );

        let installed = registry.find_installed();
        assert_eq!(installed.len(), 1);

        assert!(registry.mark_uninstalled("1").is_ok());
        assert!(!registry.get("1").unwrap().installed);
        assert!(registry.get("1").unwrap().install_path.is_none());
    }
}

// Implement SkillDiscovery trait for SkillRegistry
impl SkillDiscovery for SkillRegistry {
    fn search_by_name(&self, query: &str) -> Vec<&Skill> {
        self.find_by_name(query)
    }

    fn search_by_category(&self, category: &SkillCategory) -> Vec<&Skill> {
        self.find_by_category(category)
    }

    fn search_by_tag(&self, tag: &str) -> Vec<&Skill> {
        self.find_by_tag(tag)
    }

    fn search_by_tags(&self, tags: &[&str]) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|skill| {
                tags.iter().all(|tag| {
                    skill
                        .metadata
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase() == tag.to_lowercase())
                })
            })
            .collect()
    }

    fn list_by_name(&self) -> Vec<&Skill> {
        let mut skills = self.list_all();
        skills.sort_by_key(|s| &s.metadata.name);
        skills
    }

    fn list_by_category(&self) -> Vec<&Skill> {
        let mut skills = self.list_all();
        skills.sort_by_key(|s| s.metadata.categories.first().cloned());
        skills
    }

    fn list_all(&self) -> Vec<&Skill> {
        self.skills.values().collect()
    }
}

// Implement SkillFilter trait for SkillRegistry
impl SkillFilter for SkillRegistry {
    fn filter_by<F>(&self, predicate: F) -> Vec<&Skill>
    where
        F: Fn(&Skill) -> bool,
    {
        self.skills.values().filter(|skill| predicate(skill)).collect()
    }

    fn installed(&self) -> Vec<&Skill> {
        self.find_installed()
    }

    fn not_installed(&self) -> Vec<&Skill> {
        self.find_not_installed()
    }

    fn by_author(&self, author: &str) -> Vec<&Skill> {
        let query = author.to_lowercase();
        self.skills
            .values()
            .filter(|skill| skill.metadata.author.to_lowercase() == query)
            .collect()
    }
}

// Add query method to SkillRegistry
impl SkillRegistry {
    /// Search skills using a SkillQuery
    pub fn search_by_query(&self, query: &SkillQuery) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|skill| query.matches(skill))
            .collect()
    }
}

// Implement RatingQuery trait for SkillRegistry
impl RatingQuery for SkillRegistry {
    fn find_by_min_rating(&self, min_rating: f64) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|skill| {
                skill
                    .rating
                    .as_ref()
                    .map_or(false, |r| r.average >= min_rating)
            })
            .collect()
    }

    fn find_by_rating_and_reviews(&self, min_rating: f64, min_reviews: u32) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|skill| {
                skill
                    .rating
                    .as_ref()
                    .map_or(false, |r| r.average >= min_rating && r.count >= min_reviews)
            })
            .collect()
    }

    fn top_rated(&self, n: usize) -> Vec<&Skill> {
        let mut rated_skills: Vec<&Skill> = self
            .skills
            .values()
            .filter(|skill| skill.rating.as_ref().map_or(false, |r| r.has_ratings()))
            .collect();

        // Sort by average rating descending, then by review count descending
        rated_skills.sort_by(|a, b| {
            let avg_a = a.rating.as_ref().map_or(0.0, |r| r.average);
            let avg_b = b.rating.as_ref().map_or(0.0, |r| r.average);
            let count_a = a.rating.as_ref().map_or(0, |r| r.count);
            let count_b = b.rating.as_ref().map_or(0, |r| r.count);

            avg_b
                .partial_cmp(&avg_a)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| count_b.cmp(&count_a))
        });

        rated_skills.into_iter().take(n).collect()
    }

    fn find_unrated(&self) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|skill| !skill.rating.as_ref().map_or(false, |r| r.has_ratings()))
            .collect()
    }

    fn find_with_reviews(&self) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|skill| {
                skill
                    .rating
                    .as_ref()
                    .map_or(false, |r| !r.reviews.is_empty())
            })
            .collect()
    }
}
