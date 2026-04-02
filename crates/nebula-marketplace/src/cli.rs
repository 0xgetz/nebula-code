//! Marketplace CLI interface for browsing and managing skills
//!
//! This module provides a command-line interface for interacting with the skill marketplace.
//! It supports listing, searching, installing, uninstalling, rating, and reviewing skills.

use crate::discovery::{SkillDiscovery, SkillFilter, SkillQuery};
use crate::rating::{Rating, RatingQuery, Review, SkillRatable};
use crate::registry::SkillRegistry;
use crate::types::{MarketplaceError, Skill, SkillCategory};


/// Commands supported by the marketplace CLI
#[derive(Debug, Clone)]
pub enum Command {
    /// List all skills, optionally filtered by category or installed status
    List {
        category: Option<SkillCategory>,
        installed: Option<bool>,
    },
    /// Search skills by name, tags, or author
    Search {
        query: String,
        tags: Vec<String>,
        author: Option<String>,
    },
    /// Install a skill by ID
    Install {
        id: String,
        path: Option<String>,
    },
    /// Uninstall a skill by ID
    Uninstall {
        id: String,
    },
    /// Rate a skill (1-5 stars)
    Rate {
        id: String,
        rating: Rating,
    },
    /// Add a review to a skill
    Review {
        id: String,
        rating: Rating,
        comment: Option<String>,
        author: String,
    },
    /// Show details of a specific skill
    Show {
        id: String,
    },
    /// Show top rated skills
    TopRated {
        n: usize,
    },
    /// Help
    Help,
}

/// Marketplace CLI interface
pub struct MarketplaceCLI {
    registry: SkillRegistry,
}

impl MarketplaceCLI {
    /// Create a new CLI with an empty registry
    pub fn new() -> Self {
        Self {
            registry: SkillRegistry::new(),
        }
    }

    /// Create a new CLI with a provided registry
    pub fn with_registry(registry: SkillRegistry) -> Self {
        Self { registry }
    }

    /// Get a reference to the registry
    pub fn registry(&self) -> &SkillRegistry {
        &self.registry
    }

    /// Get a mutable reference to the registry
    pub fn registry_mut(&mut self) -> &mut SkillRegistry {
        &mut self.registry
    }

    /// Execute a command and return the output
    pub fn execute(&mut self, command: Command) -> Result<String, MarketplaceError> {
        match command {
            Command::List { category, installed } => self.handle_list(category, installed),
            Command::Search {
                query,
                tags,
                author,
            } => self.handle_search(query, tags, author),
            Command::Install { id, path } => self.handle_install(&id, path),
            Command::Uninstall { id } => self.handle_uninstall(&id),
            Command::Rate { id, rating } => self.handle_rate(&id, rating),
            Command::Review {
                id,
                rating,
                comment,
                author,
            } => self.handle_review(&id, rating, comment, &author),
            Command::Show { id } => self.handle_show(&id),
            Command::TopRated { n } => self.handle_top_rated(n),
            Command::Help => Ok(self.format_help()),
        }
    }

    fn handle_list(
        &self,
        category: Option<SkillCategory>,
        installed: Option<bool>,
    ) -> Result<String, MarketplaceError> {
        let skills: Vec<&Skill> = match (&category, installed) {
            (None, None) => self.registry.list_all(),
            (Some(cat), None) => self.registry.search_by_category(cat),
            (None, Some(true)) => self.registry.installed(),
            (None, Some(false)) => self.registry.not_installed(),
            (Some(cat), Some(installed_flag)) => {
                let mut skills = self.registry.search_by_category(cat);
                skills.retain(|s| s.installed == installed_flag);
                skills
            }
        };

        Ok(self.format_skill_list(&skills, "Skills"))
    }

    fn handle_search(
        &self,
        query: String,
        tags: Vec<String>,
        author: Option<String>,
    ) -> Result<String, MarketplaceError> {
        let mut skill_query = SkillQuery::new();

        if !query.is_empty() {
            skill_query = skill_query.with_name(&query);
        }

        for tag in &tags {
            skill_query = skill_query.with_tag(tag);
        }

        if let Some(author_name) = &author {
            skill_query = skill_query.with_author(author_name);
        }

        let skills: Vec<&Skill> = self.registry.search_by_query(&skill_query);
        Ok(self.format_skill_list(&skills, &format!("Search results for '{}'", query)))
    }

    fn handle_install(&mut self, id: &str, path: Option<String>) -> Result<String, MarketplaceError> {
        if !self.registry.contains(id) {
            return Err(MarketplaceError::SkillNotFound(id.to_string()));
        }

        self.registry.mark_installed(id, path.clone())?;
        let path_str = path.unwrap_or_else(|| "<default>".to_string());
        Ok(format!("Successfully installed skill '{}' to {}", id, path_str))
    }

    fn handle_uninstall(&mut self, id: &str) -> Result<String, MarketplaceError> {
        if !self.registry.contains(id) {
            return Err(MarketplaceError::SkillNotFound(id.to_string()));
        }

        self.registry.mark_uninstalled(id)?;
        Ok(format!("Successfully uninstalled skill '{}'", id))
    }

    fn handle_rate(&mut self, id: &str, rating: Rating) -> Result<String, MarketplaceError> {
        if !self.registry.contains(id) {
            return Err(MarketplaceError::SkillNotFound(id.to_string()));
        }

        // Initialize rating if not present
        {
            let skill = self.registry.get_mut(id).unwrap();
            if skill.rating.is_none() {
                skill.rating = Some(crate::rating::SkillRating::new());
            }
        }

        // Add the rating
        {
            let skill = self.registry.get_mut(id).unwrap();
            if let Some(ref mut skill_rating) = skill.rating {
                skill_rating.add_rating(rating);
            }
        }

        let skill = self.registry.get(id).unwrap();
        let avg = skill.average_rating();
        Ok(format!(
            "Rated skill '{}' {} star(s). New average: {:.1}",
            id,
            rating.value(),
            avg
        ))
    }

    fn handle_review(
        &mut self,
        id: &str,
        rating: Rating,
        comment: Option<String>,
        author: &str,
    ) -> Result<String, MarketplaceError> {
        if !self.registry.contains(id) {
            return Err(MarketplaceError::SkillNotFound(id.to_string()));
        }

        // Initialize rating if not present
        {
            let skill = self.registry.get_mut(id).unwrap();
            if skill.rating.is_none() {
                skill.rating = Some(crate::rating::SkillRating::new());
            }
        }

        // Add the review
        {
            let skill = self.registry.get_mut(id).unwrap();
            if let Some(ref mut skill_rating) = skill.rating {
                let timestamp = chrono::Utc::now().to_rfc3339();
                let mut review = Review::new(rating, author.to_string(), timestamp);
                if let Some(c) = comment {
                    review = review.with_comment(c);
                }
                skill_rating.add_review(review);
            }
        }

        let skill = self.registry.get(id).unwrap();
        let avg = skill.average_rating();
        let review_count = skill
            .rating
            .as_ref()
            .map(|r| r.reviews.len())
            .unwrap_or(0);
        Ok(format!(
            "Added review for skill '{}'. Average: {:.1}, Reviews: {}",
            id, avg, review_count
        ))
    }

    fn handle_show(&self, id: &str) -> Result<String, MarketplaceError> {
        let skill = self
            .registry
            .get(id)
            .ok_or_else(|| MarketplaceError::SkillNotFound(id.to_string()))?;
        Ok(self.format_skill_detail(skill))
    }

    fn handle_top_rated(&self, n: usize) -> Result<String, MarketplaceError> {
        let skills = self.registry.top_rated(n);
        Ok(self.format_skill_list(&skills, &format!("Top {} Rated Skills", n)))
    }

    fn format_help(&self) -> String {
        vec![
            "Nebula Marketplace CLI",
            "",
            "Commands:",
            "  list [category] [installed]  - List skills (optionally filtered)",
            "  search <query> [tags...]     - Search skills by name and tags",
            "  install <id> [path]          - Install a skill",
            "  uninstall <id>               - Uninstall a skill",
            "  rate <id> <1-5>              - Rate a skill (1-5 stars)",
            "  review <id> <1-5> [comment]  - Add a review with rating",
            "  show <id>                    - Show skill details",
            "  top-rated [n]                - Show top N rated skills",
            "  help                         - Show this help",
            "",
            "Examples:",
            "  list --category code-generation",
            "  search \"async\" --tags rust,async",
            "  install my-skill --path /opt/skills/my-skill",
            "  rate my-skill 5",
            "  review my-skill 4 \"Great skill!\" --author alice",
        ]
        .join("\n")
    }

    fn format_skill_list(&self, skills: &[&Skill], title: &str) -> String {
        if skills.is_empty() {
            return format!("{}\n  No skills found.", title);
        }

        let mut output = vec![format!("{} ({} skill(s))", title, skills.len()), "".to_string()];

        for skill in skills {
            let installed = if skill.installed { "[installed]" } else { "" };
            let rating_str = skill
                .rating
                .as_ref()
                .map(|r| format!(" ({:.1}★, {} reviews)", r.average, r.reviews.len()))
                .unwrap_or_default();

            output.push(format!(
                "  {:30} {} - {}{}",
                skill.metadata.name, installed, skill.id, rating_str
            ));
            output.push(format!("    {}", skill.metadata.description));
            if !skill.metadata.categories.is_empty() {
                let cats: Vec<String> = skill
                    .metadata
                    .categories
                    .iter()
                    .map(|c| c.to_string())
                    .collect();
                output.push(format!("    Categories: {}", cats.join(", ")));
            }
            output.push("".to_string());
        }

        output.join("\n")
    }

    fn format_skill_detail(&self, skill: &Skill) -> String {
        let mut output = vec![
            format!("Skill: {}", skill.metadata.name),
            format!("ID: {}", skill.id),
            format!("Description: {}", skill.metadata.description),
            format!("Author: {}", skill.metadata.author),
            format!("Version: {}", skill.metadata.version),
            format!(
                "Installed: {}",
                if skill.installed { "Yes" } else { "No" }
            ),
        ];

        if let Some(path) = &skill.install_path {
            output.push(format!("Install Path: {}", path));
        }

        if !skill.metadata.categories.is_empty() {
            let cats: Vec<String> = skill
                .metadata
                .categories
                .iter()
                .map(|c| c.to_string())
                .collect();
            output.push(format!("Categories: {}", cats.join(", ")));
        }

        if !skill.metadata.tags.is_empty() {
            output.push(format!("Tags: {}", skill.metadata.tags.join(", ")));
        }

        if let Some(ref rating) = skill.rating {
            output.push(format!("Rating: {:.1}★ ({} ratings)", rating.average, rating.count));
            if !rating.reviews.is_empty() {
                output.push("Reviews:".to_string());
                for review in &rating.reviews {
                    let comment = review
                        .comment
                        .as_ref()
                        .map(|c| format!(" - \"{}\"", c))
                        .unwrap_or_default();
                    output.push(format!(
                        "  {}★ by {} ({}){}",
                        review.rating.value(),
                        review.author,
                        review.timestamp,
                        comment
                    ));
                }
            }
        }

        output.join("\n")
    }
}

impl Default for MarketplaceCLI {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse command-line arguments into a Command
pub fn parse_command(args: &[String]) -> Result<Command, String> {
    if args.is_empty() {
        return Err("No command provided".to_string());
    }

    let cmd = &args[0];
    match cmd.as_str() {
        "list" => {
            let mut category = None;
            let mut installed = None;

            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--category" | "-c" => {
                        i += 1;
                        if i >= args.len() {
                            return Err("Missing category value".to_string());
                        }
                        category = Some(parse_category(&args[i])?);
                    }
                    "--installed" => {
                        installed = Some(true);
                    }
                    "--not-installed" => {
                        installed = Some(false);
                    }
                    _ => {
                        // Try to parse as category
                        if let Ok(cat) = parse_category(&args[i]) {
                            category = Some(cat);
                        } else {
                            return Err(format!("Unknown argument: {}", args[i]));
                        }
                    }
                }
                i += 1;
            }

            Ok(Command::List {
                category,
                installed,
            })
        }
        "search" => {
            if args.len() < 2 {
                return Err("Search requires a query".to_string());
            }

            let mut query = String::new();
            let mut tags = Vec::new();
            let mut author = None;
            let mut i = 1;

            while i < args.len() {
                match args[i].as_str() {
                    "--tags" | "-t" => {
                        i += 1;
                        if i >= args.len() {
                            return Err("Missing tags value".to_string());
                        }
                        tags = args[i].split(',').map(|s| s.trim().to_string()).collect();
                    }
                    "--author" | "-a" => {
                        i += 1;
                        if i >= args.len() {
                            return Err("Missing author value".to_string());
                        }
                        author = Some(args[i].clone());
                    }
                    arg => {
                        if !arg.starts_with('-') {
                            if !query.is_empty() {
                                query.push(' ');
                            }
                            query.push_str(arg);
                        } else {
                            return Err(format!("Unknown argument: {}", arg));
                        }
                    }
                }
                i += 1;
            }

            if query.is_empty() && tags.is_empty() && author.is_none() {
                return Err("Search requires at least a query, tags, or author".to_string());
            }

            Ok(Command::Search {
                query,
                tags,
                author,
            })
        }
        "install" => {
            if args.len() < 2 {
                return Err("Install requires a skill ID".to_string());
            }

            let id = args[1].clone();
            let path = if args.len() > 2 && !args[2].starts_with('-') {
                Some(args[2].clone())
            } else {
                None
            };

            Ok(Command::Install { id, path })
        }
        "uninstall" => {
            if args.len() < 2 {
                return Err("Uninstall requires a skill ID".to_string());
            }

            Ok(Command::Uninstall {
                id: args[1].clone(),
            })
        }
        "rate" => {
            if args.len() < 3 {
                return Err("Rate requires a skill ID and rating (1-5)".to_string());
            }

            let id = args[1].clone();
            let rating_value: u8 = args[2]
                .parse()
                .map_err(|_| "Rating must be a number between 1 and 5".to_string())?;
            let rating = Rating::new(rating_value)
                .map_err(|_| "Rating must be between 1 and 5".to_string())?;

            Ok(Command::Rate { id, rating })
        }
        "review" => {
            if args.len() < 3 {
                return Err("Review requires a skill ID, rating (1-5), and optional comment".to_string());
            }

            let id = args[1].clone();
            let rating_value: u8 = args[2]
                .parse()
                .map_err(|_| "Rating must be a number between 1 and 5".to_string())?;
            let rating = Rating::new(rating_value)
                .map_err(|_| "Rating must be between 1 and 5".to_string())?;

            let comment = if args.len() > 3 && !args[3].starts_with('-') {
                Some(args[3].clone())
            } else {
                None
            };

            let author = "anonymous".to_string(); // Default author

            Ok(Command::Review {
                id,
                rating,
                comment,
                author,
            })
        }
        "show" => {
            if args.len() < 2 {
                return Err("Show requires a skill ID".to_string());
            }

            Ok(Command::Show {
                id: args[1].clone(),
            })
        }
        "top-rated" => {
            let n: usize = if args.len() > 1 {
                args[1].parse().unwrap_or(10)
            } else {
                10
            };

            Ok(Command::TopRated { n })
        }
        "help" | "--help" | "-h" => Ok(Command::Help),
        _ => Err(format!("Unknown command: {}", cmd)),
    }
}

/// Parse a category string into a SkillCategory
pub fn parse_category(s: &str) -> Result<SkillCategory, String> {
    let lower = s.to_lowercase().replace('-', " ").replace('_', " ");
    match lower.as_str() {
        "codegeneration" | "code generation" => Ok(SkillCategory::CodeGeneration),
        "dataanalysis" | "data analysis" => Ok(SkillCategory::DataAnalysis),
        "automation" => Ok(SkillCategory::Automation),
        "communication" => Ok(SkillCategory::Communication),
        "research" => Ok(SkillCategory::Research),
        "development" => Ok(SkillCategory::Development),
        "devops" | "dev ops" => Ok(SkillCategory::DevOps),
        "security" => Ok(SkillCategory::Security),
        "monitoring" => Ok(SkillCategory::Monitoring),
        "testing" => Ok(SkillCategory::Testing),
        "documentation" => Ok(SkillCategory::Documentation),
        "other" => Ok(SkillCategory::Other),
        _ => Err(format!("Unknown category: {}", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{SkillManifest, SkillMetadata};

    fn create_test_skill(id: &str, name: &str) -> Skill {
        let metadata = SkillMetadata::new(
            name.to_string(),
            "A test skill".to_string(),
            "test-author".to_string(),
        )
        .with_categories(vec![SkillCategory::CodeGeneration])
        .with_tags(vec!["test".to_string()]);
        let manifest = SkillManifest::new("main.rs".to_string(), "rust".to_string());
        Skill::new(id.to_string(), metadata, manifest)
    }

    #[test]
    fn test_cli_new() {
        let cli = MarketplaceCLI::new();
        assert!(cli.registry().is_empty());
    }

    #[test]
    fn test_cli_list_empty() {
        let mut cli = MarketplaceCLI::new();
        let result = cli.execute(Command::List {
            category: None,
            installed: None,
        });
        assert!(result.is_ok());
        assert!(result.unwrap().contains("No skills found"));
    }

    #[test]
    fn test_cli_list_with_skills() {
        let mut cli = MarketplaceCLI::new();
        cli.registry
            .register(create_test_skill("test-1", "Test Skill 1"))
            .unwrap();
        cli.registry
            .register(create_test_skill("test-2", "Test Skill 2"))
            .unwrap();

        let result = cli.execute(Command::List {
            category: None,
            installed: None,
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("2 skill(s)"));
        assert!(output.contains("Test Skill 1"));
        assert!(output.contains("Test Skill 2"));
    }

    #[test]
    fn test_cli_search() {
        let mut cli = MarketplaceCLI::new();
        cli.registry
            .register(create_test_skill("test-1", "Async Helper"))
            .unwrap();
        cli.registry
            .register(create_test_skill("test-2", "Data Processor"))
            .unwrap();

        let result = cli.execute(Command::Search {
            query: "async".to_string(),
            tags: vec![],
            author: None,
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Async Helper"));
        assert!(!output.contains("Data Processor"));
    }

    #[test]
    fn test_cli_install_uninstall() {
        let mut cli = MarketplaceCLI::new();
        cli.registry
            .register(create_test_skill("test-1", "Test Skill"))
            .unwrap();

        // Install
        let result = cli.execute(Command::Install {
            id: "test-1".to_string(),
            path: Some("/opt/skills/test".to_string()),
        });
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Successfully installed"));

        // Verify installed
        assert!(cli.registry.get("test-1").unwrap().installed);
        assert_eq!(
            cli.registry.get("test-1").unwrap().install_path,
            Some("/opt/skills/test".to_string())
        );

        // Uninstall
        let result = cli.execute(Command::Uninstall {
            id: "test-1".to_string(),
        });
        assert!(result.is_ok());
        assert!(result.unwrap().contains("Successfully uninstalled"));

        // Verify uninstalled
        assert!(!cli.registry.get("test-1").unwrap().installed);
        assert!(cli.registry.get("test-1").unwrap().install_path.is_none());
    }

    #[test]
    fn test_cli_rate() {
        let mut cli = MarketplaceCLI::new();
        cli.registry
            .register(create_test_skill("test-1", "Test Skill"))
            .unwrap();

        let rating = Rating::new(5).unwrap();
        let result = cli.execute(Command::Rate {
            id: "test-1".to_string(),
            rating,
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Rated skill"));
        assert!(output.contains("5"));

        // Verify rating was added
        let skill = cli.registry.get("test-1").unwrap();
        assert!(skill.rating.is_some());
        assert_eq!(skill.rating.as_ref().unwrap().count, 1);
        assert_eq!(skill.rating.as_ref().unwrap().average, 5.0);
    }

    #[test]
    fn test_cli_show() {
        let mut cli = MarketplaceCLI::new();
        cli.registry
            .register(create_test_skill("test-1", "Test Skill"))
            .unwrap();

        let result = cli.execute(Command::Show {
            id: "test-1".to_string(),
        });
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Test Skill"));
        assert!(output.contains("test-1"));
    }

    #[test]
    fn test_cli_show_not_found() {
        let mut cli = MarketplaceCLI::new();
        let result = cli.execute(Command::Show {
            id: "non-existent".to_string(),
        });
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MarketplaceError::SkillNotFound(_)
        ));
    }

    #[test]
    fn test_parse_command_list() {
        let args = vec!["list".to_string()];
        let cmd = parse_command(&args);
        assert!(cmd.is_ok());
        match cmd.unwrap() {
            Command::List {
                category,
                installed,
            } => {
                assert!(category.is_none());
                assert!(installed.is_none());
            }
            _ => panic!("Expected List command"),
        }
    }

    #[test]
    fn test_parse_command_search() {
        let args = vec![
            "search".to_string(),
            "async helper".to_string(),
            "--tags".to_string(),
            "rust,async".to_string(),
        ];
        let cmd = parse_command(&args);
        assert!(cmd.is_ok());
        match cmd.unwrap() {
            Command::Search { query, tags, .. } => {
                assert_eq!(query, "async helper");
                assert_eq!(tags, vec!["rust", "async"]);
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_parse_command_rate() {
        let args = vec![
            "rate".to_string(),
            "my-skill".to_string(),
            "4".to_string(),
        ];
        let cmd = parse_command(&args);
        assert!(cmd.is_ok());
        match cmd.unwrap() {
            Command::Rate { id, rating } => {
                assert_eq!(id, "my-skill");
                assert_eq!(rating.value(), 4);
            }
            _ => panic!("Expected Rate command"),
        }
    }

    #[test]
    fn test_parse_command_help() {
        let args = vec!["help".to_string()];
        let cmd = parse_command(&args);
        assert!(cmd.is_ok());
        assert!(matches!(cmd.unwrap(), Command::Help));
    }

    #[test]
    fn test_parse_category() {
        assert_eq!(
            parse_category("code-generation").unwrap(),
            SkillCategory::CodeGeneration
        );
        assert_eq!(
            parse_category("dataanalysis").unwrap(),
            SkillCategory::DataAnalysis
        );
        assert!(parse_category("unknown").is_err());
    }
}
