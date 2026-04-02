use crate::types::{Skill, SkillCategory};

/// Trait for discovering skills by various criteria
pub trait SkillDiscovery {
    /// Search skills by name (case-insensitive partial match)
    fn search_by_name(&self, query: &str) -> Vec<&Skill>;

    /// Search skills by category
    fn search_by_category(&self, category: &SkillCategory) -> Vec<&Skill>;

    /// Search skills by tag
    fn search_by_tag(&self, tag: &str) -> Vec<&Skill>;

    /// Search skills by multiple tags (AND logic)
    fn search_by_tags(&self, tags: &[&str]) -> Vec<&Skill>;

    /// Get skills sorted by name
    fn list_by_name(&self) -> Vec<&Skill>;

    /// Get skills sorted by category
    fn list_by_category(&self) -> Vec<&Skill>;

    /// Get all skills
    fn list_all(&self) -> Vec<&Skill>;
}

/// Trait for filtering skills
pub trait SkillFilter {
    /// Filter skills by a predicate
    fn filter_by<F>(&self, predicate: F) -> Vec<&Skill>
    where
        F: Fn(&Skill) -> bool;

    /// Get installed skills
    fn installed(&self) -> Vec<&Skill>;

    /// Get not installed skills
    fn not_installed(&self) -> Vec<&Skill>;

    /// Get skills by author
    fn by_author(&self, author: &str) -> Vec<&Skill>;
}

/// Query builder for complex skill searches
#[derive(Debug, Default)]
pub struct SkillQuery {
    name: Option<String>,
    categories: Vec<SkillCategory>,
    tags: Vec<String>,
    author: Option<String>,
    installed: Option<bool>,
}

impl SkillQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    pub fn with_category(mut self, category: SkillCategory) -> Self {
        self.categories.push(category);
        self
    }

    pub fn with_categories(mut self, categories: Vec<SkillCategory>) -> Self {
        self.categories = categories;
        self
    }

    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_author(mut self, author: &str) -> Self {
        self.author = Some(author.to_string());
        self
    }

    pub fn with_installed(mut self, installed: bool) -> Self {
        self.installed = Some(installed);
        self
    }

    /// Check if a skill matches this query
    pub fn matches(&self, skill: &Skill) -> bool {
        // Check name
        if let Some(name) = &self.name {
            if !skill
                .metadata
                .name
                .to_lowercase()
                .contains(&name.to_lowercase())
            {
                return false;
            }
        }

        // Check categories (OR logic)
        if !self.categories.is_empty() {
            if !self
                .categories
                .iter()
                .any(|cat| skill.metadata.categories.contains(cat))
            {
                return false;
            }
        }

        // Check tags (AND logic)
        if !self.tags.is_empty() {
            for tag in &self.tags {
                if !skill
                    .metadata
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase() == tag.to_lowercase())
                {
                    return false;
                }
            }
        }

        // Check author
        if let Some(author) = &self.author {
            if skill.metadata.author.to_lowercase() != author.to_lowercase() {
                return false;
            }
        }

        // Check installed status
        if let Some(installed) = self.installed {
            if skill.installed != installed {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SkillManifest, SkillMetadata};

    fn create_test_skill(id: &str, name: &str, tags: Vec<&str>, categories: Vec<SkillCategory>) -> Skill {
        let mut metadata = SkillMetadata::new(name.to_string(), "desc".to_string(), "author".to_string());
        metadata.tags = tags.iter().map(|s| s.to_string()).collect();
        metadata.categories = categories;
        let manifest = SkillManifest::new("main.rs".to_string(), "rust".to_string());
        Skill::new(id.to_string(), metadata, manifest)
    }

    #[test]
    fn test_query_name_match() {
        let skill = create_test_skill("1", "Test Skill", vec![], vec![]);
        let query = SkillQuery::new().with_name("test");
        assert!(query.matches(&skill));

        let query = SkillQuery::new().with_name("nonexistent");
        assert!(!query.matches(&skill));
    }

    #[test]
    fn test_query_category_match() {
        let skill = create_test_skill(
            "1",
            "Test",
            vec![],
            vec![SkillCategory::CodeGeneration, SkillCategory::Development],
        );

        let query = SkillQuery::new().with_category(SkillCategory::CodeGeneration);
        assert!(query.matches(&skill));

        let query = SkillQuery::new().with_category(SkillCategory::Automation);
        assert!(!query.matches(&skill));
    }

    #[test]
    fn test_query_tag_match() {
        let skill = create_test_skill("1", "Test", vec!["rust", "async"], vec![]);

        let query = SkillQuery::new().with_tag("rust");
        assert!(query.matches(&skill));

        let query = SkillQuery::new().with_tag("python");
        assert!(!query.matches(&skill));

        // Test AND logic for multiple tags
        let query = SkillQuery::new().with_tag("rust").with_tag("async");
        assert!(query.matches(&skill));

        let query = SkillQuery::new().with_tag("rust").with_tag("python");
        assert!(!query.matches(&skill));
    }

    #[test]
    fn test_query_combined() {
        let skill = create_test_skill(
            "1",
            "Async Helper",
            vec!["rust", "async"],
            vec![SkillCategory::Development],
        );

        let query = SkillQuery::new()
            .with_name("async")
            .with_category(SkillCategory::Development)
            .with_tag("rust");
        assert!(query.matches(&skill));

        let query = SkillQuery::new()
            .with_name("nonexistent")
            .with_category(SkillCategory::Development);
        assert!(!query.matches(&skill));
    }

    #[test]
    fn test_query_installed_filter() {
        let mut skill = create_test_skill("1", "Test", vec![], vec![]);
        skill.installed = true;

        let query = SkillQuery::new().with_installed(true);
        assert!(query.matches(&skill));

        let query = SkillQuery::new().with_installed(false);
        assert!(!query.matches(&skill));
    }
}
