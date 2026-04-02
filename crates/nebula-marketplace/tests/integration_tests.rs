//! Integration tests for nebula-marketplace crate
//!
//! These tests cover:
//! 1. Full workflow: create registry, add skills, search, install, rate, review
//! 2. Edge cases: duplicate installs, invalid ratings, uninstalling non-existent skills
//! 3. Rating aggregation and top-rated queries
//! 4. CLI command parsing and execution

use nebula_marketplace::rating::{Rating, RatingQuery, Review, SkillRating};
use nebula_marketplace::registry::SkillRegistry;
use nebula_marketplace::types::{
    MarketplaceError, Skill, SkillCategory, SkillManifest, SkillMetadata, SkillVersion,
};
use nebula_marketplace::{MarketplaceCLI, Command, parse_command, SkillQuery};

// ============================================================================
// Helper functions
// ============================================================================

fn create_skill(id: &str, name: &str, categories: Vec<SkillCategory>, tags: Vec<String>) -> Skill {
    let metadata = SkillMetadata::new(
        name.to_string(),
        format!("Description for {}", name),
        "test-author".to_string(),
    )
    .with_categories(categories)
    .with_tags(tags)
    .with_version(SkillVersion::new(1, 0, 0));

    let manifest = SkillManifest::new("main.rs".to_string(), "rust".to_string());
    Skill::new(id.to_string(), metadata, manifest)
}

fn create_skill_with_rating(id: &str, name: &str, rating: SkillRating) -> Skill {
    let mut skill = create_skill(
        id,
        name,
        vec![SkillCategory::CodeGeneration],
        vec!["test".to_string()],
    );
    skill.rating = Some(rating);
    skill
}

// ============================================================================
// 1. Full workflow tests
// ============================================================================

#[test]
fn test_full_workflow_create_registry_and_add_skills() {
    // Create a new registry
    let mut registry = SkillRegistry::new();
    assert!(registry.is_empty());

    // Add multiple skills
    let skill1 = create_skill("skill-1", "Async Helper", vec![SkillCategory::CodeGeneration], vec!["async".to_string(), "rust".to_string()]);
    let skill2 = create_skill("skill-2", "Data Processor", vec![SkillCategory::DataAnalysis], vec!["data".to_string()]);
    let skill3 = create_skill("skill-3", "CLI Builder", vec![SkillCategory::Development], vec!["cli".to_string()]);

    assert!(registry.register(skill1).is_ok());
    assert!(registry.register(skill2).is_ok());
    assert!(registry.register(skill3).is_ok());

    assert_eq!(registry.len(), 3);
    assert!(!registry.is_empty());
}

#[test]
fn test_full_workflow_search_skills() {
    let mut registry = SkillRegistry::new();

    registry.register(create_skill("1", "Async Helper", vec![SkillCategory::CodeGeneration], vec!["async".to_string(), "rust".to_string()])).unwrap();
    registry.register(create_skill("2", "Data Processor", vec![SkillCategory::DataAnalysis], vec!["data".to_string()])).unwrap();
    registry.register(create_skill("3", "CLI Builder", vec![SkillCategory::Development], vec!["cli".to_string()])).unwrap();

    // Search by name
    let results = registry.find_by_name("async");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");

    // Search by category
    let results = registry.find_by_category(&SkillCategory::DataAnalysis);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "2");

    // Search by tag
    let results = registry.find_by_tag("rust");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "1");

    // Search using SkillQuery
    let query = SkillQuery::new().with_name("builder");
    let results = registry.search_by_query(&query);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "3");
}

#[test]
fn test_full_workflow_install_and_uninstall() {
    let mut registry = SkillRegistry::new();

    let skill = create_skill("test-skill", "Test Skill", vec![SkillCategory::CodeGeneration], vec![]);
    registry.register(skill).unwrap();

    // Verify not installed initially
    assert!(!registry.get("test-skill").unwrap().installed);

    // Install the skill
    assert!(registry.mark_installed("test-skill", Some("/opt/skills/test".to_string())).is_ok());
    assert!(registry.get("test-skill").unwrap().installed);
    assert_eq!(registry.get("test-skill").unwrap().install_path, Some("/opt/skills/test".to_string()));

    // Find installed skills
    let installed = registry.find_installed();
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].id, "test-skill");

    // Uninstall the skill
    assert!(registry.mark_uninstalled("test-skill").is_ok());
    assert!(!registry.get("test-skill").unwrap().installed);
    assert!(registry.get("test-skill").unwrap().install_path.is_none());

    // Find not installed skills
    let not_installed = registry.find_not_installed();
    assert_eq!(not_installed.len(), 1);
}

#[test]
fn test_full_workflow_rate_and_review() {
    let mut registry = SkillRegistry::new();

    let mut skill = create_skill("rate-test", "Rate Test Skill", vec![SkillCategory::CodeGeneration], vec![]);
    skill.rating = Some(SkillRating::new());
    registry.register(skill).unwrap();

    // Add ratings
    {
        let s = registry.get_mut("rate-test").unwrap();
        if let Some(ref mut rating) = s.rating {
            rating.add_rating(Rating::new(5).unwrap());
            rating.add_rating(Rating::new(4).unwrap());
            rating.add_rating(Rating::new(5).unwrap());
        }
    }

    let skill = registry.get("rate-test").unwrap();
    assert!(skill.rating.is_some());
    let skill_rating = skill.rating.as_ref().unwrap();
    assert_eq!(skill_rating.count, 3);
    assert!((skill_rating.average - 4.666666).abs() < 0.001);

    // Add a review
    {
        let s = registry.get_mut("rate-test").unwrap();
        if let Some(ref mut rating) = s.rating {
            let review = Review::new(
                Rating::new(3).unwrap(),
                "alice".to_string(),
                "2024-01-15T10:30:00Z".to_string(),
            )
            .with_comment("Good but could be better".to_string());
            rating.add_review(review);
        }
    }

    let skill = registry.get("rate-test").unwrap();
    let skill_rating = skill.rating.as_ref().unwrap();
    assert_eq!(skill_rating.count, 4);
    assert_eq!(skill_rating.reviews.len(), 1);
    assert_eq!(skill_rating.reviews[0].author, "alice");
    assert!(skill_rating.reviews[0].comment.is_some());
}

// ============================================================================
// 2. Edge case tests
// ============================================================================

#[test]
fn test_edge_case_duplicate_registration() {
    let mut registry = SkillRegistry::new();

    let skill1 = create_skill("dup-skill", "Duplicate Skill", vec![], vec![]);
    let skill2 = create_skill("dup-skill", "Duplicate Skill v2", vec![], vec![]);

    assert!(registry.register(skill1).is_ok());
    let result = registry.register(skill2);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), MarketplaceError::InvalidVersion(_)));
}

#[test]
fn test_edge_case_duplicate_install() {
    let mut registry = SkillRegistry::new();

    let skill = create_skill("install-test", "Install Test", vec![], vec![]);
    registry.register(skill).unwrap();

    // Install once
    assert!(registry.mark_installed("install-test", None).is_ok());
    assert!(registry.get("install-test").unwrap().installed);

    // Installing again should succeed (idempotent) but not change state
    assert!(registry.mark_installed("install-test", None).is_ok());
    assert!(registry.get("install-test").unwrap().installed);
}

#[test]
fn test_edge_case_invalid_ratings() {
    // Test invalid rating values
    assert!(Rating::new(0).is_err());
    assert!(Rating::new(6).is_err());
    assert!(Rating::new(255).is_err());

    // Test valid rating values
    assert!(Rating::new(1).is_ok());
    assert!(Rating::new(2).is_ok());
    assert!(Rating::new(3).is_ok());
    assert!(Rating::new(4).is_ok());
    assert!(Rating::new(5).is_ok());

    // Test TryFrom
    let valid: Result<Rating, _> = 4u8.try_into();
    assert!(valid.is_ok());
    let invalid: Result<Rating, _> = 0u8.try_into();
    assert!(invalid.is_err());
}

#[test]
fn test_edge_case_uninstall_nonexistent() {
    let mut registry = SkillRegistry::new();

    let result = registry.mark_uninstalled("non-existent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), MarketplaceError::SkillNotFound(_)));

    let result = registry.mark_installed("non-existent", None);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), MarketplaceError::SkillNotFound(_)));
}

#[test]
fn test_edge_case_rate_nonexistent_skill() {
    let mut registry = SkillRegistry::new();

    let result = registry.get_mut("non-existent");
    assert!(result.is_none());
}

#[test]
fn test_edge_case_unregister_nonexistent() {
    let mut registry = SkillRegistry::new();

    let result = registry.unregister("non-existent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), MarketplaceError::SkillNotFound(_)));
}

#[test]
fn test_edge_case_empty_search() {
    let registry = SkillRegistry::new();

    let results = registry.find_by_name("anything");
    assert!(results.is_empty());

    let results = registry.find_by_category(&SkillCategory::CodeGeneration);
    assert!(results.is_empty());

    let results = registry.top_rated(10);
    assert!(results.is_empty());
}

// ============================================================================
// 3. Rating aggregation and top-rated queries
// ============================================================================

#[test]
fn test_rating_aggregation_basic() {
    let mut rating = SkillRating::new();

    rating.add_rating(Rating::new(5).unwrap());
    rating.add_rating(Rating::new(4).unwrap());
    rating.add_rating(Rating::new(3).unwrap());
    rating.add_rating(Rating::new(4).unwrap());
    rating.add_rating(Rating::new(5).unwrap());

    assert_eq!(rating.count, 5);
    assert_eq!(rating.average, 4.2);
    assert_eq!(rating.average_rounded(), 4.2);
    assert_eq!(rating.distribution.get(&5), Some(&2));
    assert_eq!(rating.distribution.get(&4), Some(&2));
    assert_eq!(rating.distribution.get(&3), Some(&1));

    // Median should be 4.0 (middle value of sorted [3,4,4,5,5])
    assert_eq!(rating.median(), Some(4.0));

    // Mode should be 4 or 5 (both appear twice, but we get one of them)
    let mode = rating.mode();
    assert!(mode == Some(4) || mode == Some(5));
}

#[test]
fn test_rating_aggregation_weighted() {
    let mut rating = SkillRating::new();

    // 10 ratings: 8 five-star, 2 four-star
    for _ in 0..8 {
        rating.add_rating(Rating::new(5).unwrap());
    }
    for _ in 0..2 {
        rating.add_rating(Rating::new(4).unwrap());
    }

    assert_eq!(rating.count, 10);
    assert_eq!(rating.average, 4.8);
    assert_eq!(rating.mode(), Some(5));
}

#[test]
fn test_top_rated_sorting() {
    let mut registry = SkillRegistry::new();

    // Create skills with different ratings
    let mut rating_a = SkillRating::new();
    rating_a.add_rating(Rating::new(5).unwrap());
    rating_a.add_rating(Rating::new(5).unwrap());
    registry.register(create_skill_with_rating("skill-a", "Skill A", rating_a)).unwrap();

    let mut rating_b = SkillRating::new();
    rating_b.add_rating(Rating::new(4).unwrap());
    rating_b.add_rating(Rating::new(5).unwrap());
    registry.register(create_skill_with_rating("skill-b", "Skill B", rating_b)).unwrap();

    let mut rating_c = SkillRating::new();
    rating_c.add_rating(Rating::new(3).unwrap());
    registry.register(create_skill_with_rating("skill-c", "Skill C", rating_c)).unwrap();

    let mut rating_d = SkillRating::new();
    rating_d.add_rating(Rating::new(5).unwrap());
    rating_d.add_rating(Rating::new(5).unwrap());
    rating_d.add_rating(Rating::new(5).unwrap());
    registry.register(create_skill_with_rating("skill-d", "Skill D", rating_d)).unwrap();

    // Get top rated (should be sorted by average desc, then by count desc)
    let top = registry.top_rated(4);
    assert_eq!(top.len(), 4);

    // Skill D (5.0 avg, 3 ratings) should be first
    assert_eq!(top[0].id, "skill-d");
    // Skill A (5.0 avg, 2 ratings) should be second
    assert_eq!(top[1].id, "skill-a");
    // Skill B (4.5 avg) should be third
    assert_eq!(top[2].id, "skill-b");
    // Skill C (3.0 avg) should be last
    assert_eq!(top[3].id, "skill-c");
}

#[test]
fn test_find_by_min_rating() {
    let mut registry = SkillRegistry::new();

    let mut rating_a = SkillRating::new();
    rating_a.add_rating(Rating::new(5).unwrap());
    registry.register(create_skill_with_rating("skill-a", "Skill A", rating_a)).unwrap();

    let mut rating_b = SkillRating::new();
    rating_b.add_rating(Rating::new(4).unwrap());
    rating_b.add_rating(Rating::new(4).unwrap());
    registry.register(create_skill_with_rating("skill-b", "Skill B", rating_b)).unwrap();

    let mut rating_c = SkillRating::new();
    rating_c.add_rating(Rating::new(3).unwrap());
    registry.register(create_skill_with_rating("skill-c", "Skill C", rating_c)).unwrap();

    let mut rating_d = SkillRating::new();
    rating_d.add_rating(Rating::new(2).unwrap());
    registry.register(create_skill_with_rating("skill-d", "Skill D", rating_d)).unwrap();

    // Find skills with rating >= 4.0
    let results = registry.find_by_min_rating(4.0);
    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|s| s.id == "skill-a"));
    assert!(results.iter().any(|s| s.id == "skill-b"));

    // Find skills with rating >= 4.5
    let results = registry.find_by_min_rating(4.5);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "skill-a");

    // Find skills with rating >= 5.0
    let results = registry.find_by_min_rating(5.0);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "skill-a");
}

#[test]
fn test_find_by_rating_and_reviews() {
    let mut registry = SkillRegistry::new();

    // Skill with high rating but only 1 review
    let mut rating_a = SkillRating::new();
    rating_a.add_rating(Rating::new(5).unwrap());
    registry.register(create_skill_with_rating("skill-a", "Skill A", rating_a)).unwrap();

    // Skill with high rating and multiple reviews
    let mut rating_b = SkillRating::new();
    rating_b.add_rating(Rating::new(5).unwrap());
    rating_b.add_rating(Rating::new(4).unwrap());
    rating_b.add_rating(Rating::new(5).unwrap());
    registry.register(create_skill_with_rating("skill-b", "Skill B", rating_b)).unwrap();

    // Find skills with rating >= 4.0 and at least 2 reviews
    let results = registry.find_by_rating_and_reviews(4.0, 2);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "skill-b");

    // Find skills with rating >= 4.0 and at least 3 reviews
    let results = registry.find_by_rating_and_reviews(4.0, 3);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "skill-b");
}

#[test]
fn test_find_unrated_skills() {
    let mut registry = SkillRegistry::new();

    let mut skill_a = create_skill("skill-a", "Skill A", vec![], vec![]);
    skill_a.rating = None;
    registry.register(skill_a).unwrap();

    let mut skill_b = create_skill("skill-b", "Skill B", vec![], vec![]);
    skill_b.rating = Some(SkillRating::new()); // Empty rating
    registry.register(skill_b).unwrap();

    let mut skill_c = create_skill("skill-c", "Skill C", vec![], vec![]);
    let mut rating_c = SkillRating::new();
    rating_c.add_rating(Rating::new(5).unwrap());
    skill_c.rating = Some(rating_c);
    registry.register(skill_c).unwrap();

    let unrated = registry.find_unrated();
    assert_eq!(unrated.len(), 2);
    assert!(unrated.iter().any(|s| s.id == "skill-a"));
    assert!(unrated.iter().any(|s| s.id == "skill-b"));
}

#[test]
fn test_find_with_reviews() {
    let mut registry = SkillRegistry::new();

    // Skill with no reviews
    let mut skill_a = create_skill("skill-a", "Skill A", vec![], vec![]);
    skill_a.rating = Some(SkillRating::new());
    registry.register(skill_a).unwrap();

    // Skill with reviews
    let mut skill_b = create_skill("skill-b", "Skill B", vec![], vec![]);
    let mut rating_b = SkillRating::new();
    let review = Review::new(
        Rating::new(5).unwrap(),
        "alice".to_string(),
        "2024-01-01T00:00:00Z".to_string(),
    )
    .with_comment("Great!".to_string());
    rating_b.add_review(review);
    skill_b.rating = Some(rating_b);
    registry.register(skill_b).unwrap();

    let with_reviews = registry.find_with_reviews();
    assert_eq!(with_reviews.len(), 1);
    assert_eq!(with_reviews[0].id, "skill-b");
}

// ============================================================================
// 4. CLI command parsing and execution tests
// ============================================================================

#[test]
fn test_cli_parse_list_command() {
    // Basic list
    let args = vec!["list".to_string()];
    let cmd = parse_command(&args).unwrap();
    assert!(matches!(cmd, Command::List { category: None, installed: None }));

    // List with category
    let args = vec!["list".to_string(), "--category".to_string(), "code-generation".to_string()];
    let cmd = parse_command(&args).unwrap();
    assert!(matches!(cmd, Command::List { category: Some(SkillCategory::CodeGeneration), installed: None }));

    // List installed only
    let args = vec!["list".to_string(), "--installed".to_string()];
    let cmd = parse_command(&args).unwrap();
    assert!(matches!(cmd, Command::List { category: None, installed: Some(true) }));

    // List not installed only
    let args = vec!["list".to_string(), "--not-installed".to_string()];
    let cmd = parse_command(&args).unwrap();
    assert!(matches!(cmd, Command::List { category: None, installed: Some(false) }));
}

#[test]
fn test_cli_parse_search_command() {
    // Search with query only
    let args = vec!["search".to_string(), "async helper".to_string()];
    let cmd = parse_command(&args).unwrap();
    match cmd {
        Command::Search { query, tags, author } => {
            assert_eq!(query, "async helper");
            assert!(tags.is_empty());
            assert!(author.is_none());
        }
        _ => panic!("Expected Search command"),
    }

    // Search with tags
    let args = vec![
        "search".to_string(),
        "rust".to_string(),
        "--tags".to_string(),
        "async,concurrency".to_string(),
    ];
    let cmd = parse_command(&args).unwrap();
    match cmd {
        Command::Search { query, tags, author } => {
            assert_eq!(query, "rust");
            assert_eq!(tags, vec!["async", "concurrency"]);
            assert!(author.is_none());
        }
        _ => panic!("Expected Search command"),
    }

    // Search with author
    let args = vec![
        "search".to_string(),
        "--author".to_string(),
        "alice".to_string(),
    ];
    let cmd = parse_command(&args).unwrap();
    match cmd {
        Command::Search { query, tags, author } => {
            assert!(query.is_empty());
            assert!(tags.is_empty());
            assert_eq!(author, Some("alice".to_string()));
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_cli_parse_install_command() {
    // Install without path
    let args = vec!["install".to_string(), "my-skill".to_string()];
    let cmd = parse_command(&args).unwrap();
    match cmd {
        Command::Install { id, path } => {
            assert_eq!(id, "my-skill");
            assert!(path.is_none());
        }
        _ => panic!("Expected Install command"),
    }

    // Install with path
    let args = vec!["install".to_string(), "my-skill".to_string(), "/opt/skills".to_string()];
    let cmd = parse_command(&args).unwrap();
    match cmd {
        Command::Install { id, path } => {
            assert_eq!(id, "my-skill");
            assert_eq!(path, Some("/opt/skills".to_string()));
        }
        _ => panic!("Expected Install command"),
    }
}

#[test]
fn test_cli_parse_rate_command() {
    let args = vec!["rate".to_string(), "my-skill".to_string(), "5".to_string()];
    let cmd = parse_command(&args).unwrap();
    match cmd {
        Command::Rate { id, rating } => {
            assert_eq!(id, "my-skill");
            assert_eq!(rating.value(), 5);
        }
        _ => panic!("Expected Rate command"),
    }
}

#[test]
fn test_cli_parse_rate_command_invalid() {
    // Invalid rating value (0)
    let args = vec!["rate".to_string(), "my-skill".to_string(), "0".to_string()];
    assert!(parse_command(&args).is_err());

    // Invalid rating value (6)
    let args = vec!["rate".to_string(), "my-skill".to_string(), "6".to_string()];
    assert!(parse_command(&args).is_err());

    // Missing rating
    let args = vec!["rate".to_string(), "my-skill".to_string()];
    assert!(parse_command(&args).is_err());
}

#[test]
fn test_cli_parse_review_command() {
    let args = vec!["review".to_string(), "my-skill".to_string(), "4".to_string(), "Great skill!".to_string()];
    let cmd = parse_command(&args).unwrap();
    match cmd {
        Command::Review { id, rating, comment, author } => {
            assert_eq!(id, "my-skill");
            assert_eq!(rating.value(), 4);
            assert_eq!(comment, Some("Great skill!".to_string()));
            assert_eq!(author, "anonymous");
        }
        _ => panic!("Expected Review command"),
    }
}

#[test]
fn test_cli_parse_show_command() {
    let args = vec!["show".to_string(), "my-skill".to_string()];
    let cmd = parse_command(&args).unwrap();
    assert!(matches!(cmd, Command::Show { id } if id == "my-skill"));
}

#[test]
fn test_cli_parse_top_rated_command() {
    // Default (10)
    let args = vec!["top-rated".to_string()];
    let cmd = parse_command(&args).unwrap();
    assert!(matches!(cmd, Command::TopRated { n: 10 }));

    // Custom number
    let args = vec!["top-rated".to_string(), "5".to_string()];
    let cmd = parse_command(&args).unwrap();
    assert!(matches!(cmd, Command::TopRated { n: 5 }));
}

#[test]
fn test_cli_parse_help_command() {
    let args = vec!["help".to_string()];
    let cmd = parse_command(&args).unwrap();
    assert!(matches!(cmd, Command::Help));

    let args = vec!["--help".to_string()];
    let cmd = parse_command(&args).unwrap();
    assert!(matches!(cmd, Command::Help));
}

#[test]
fn test_cli_execute_list() {
    let mut cli = MarketplaceCLI::new();

    cli.registry_mut().register(create_skill("1", "Skill One", vec![], vec![])).unwrap();
    cli.registry_mut().register(create_skill("2", "Skill Two", vec![], vec![])).unwrap();

    let result = cli.execute(Command::List { category: None, installed: None });
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("2 skill(s)"));
    assert!(output.contains("Skill One"));
    assert!(output.contains("Skill Two"));
}

#[test]
fn test_cli_execute_install_and_uninstall() {
    let mut cli = MarketplaceCLI::new();
    cli.registry_mut().register(create_skill("test-1", "Test Skill", vec![], vec![])).unwrap();

    // Install
    let result = cli.execute(Command::Install {
        id: "test-1".to_string(),
        path: Some("/opt/test".to_string()),
    });
    assert!(result.is_ok());
    assert!(result.unwrap().contains("Successfully installed"));

    // Verify
    assert!(cli.registry().get("test-1").unwrap().installed);

    // Uninstall
    let result = cli.execute(Command::Uninstall { id: "test-1".to_string() });
    assert!(result.is_ok());
    assert!(result.unwrap().contains("Successfully uninstalled"));

    // Verify
    assert!(!cli.registry().get("test-1").unwrap().installed);
}

#[test]
fn test_cli_execute_install_nonexistent() {
    let mut cli = MarketplaceCLI::new();

    let result = cli.execute(Command::Install {
        id: "non-existent".to_string(),
        path: None,
    });
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), MarketplaceError::SkillNotFound(_)));
}

#[test]
fn test_cli_execute_rate() {
    let mut cli = MarketplaceCLI::new();
    cli.registry_mut().register(create_skill("rate-test", "Rate Test", vec![], vec![])).unwrap();

    let rating = Rating::new(4).unwrap();
    let result = cli.execute(Command::Rate {
        id: "rate-test".to_string(),
        rating,
    });
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Rated skill"));
    assert!(output.contains("4"));

    // Verify rating was added
    let skill = cli.registry().get("rate-test").unwrap();
    assert!(skill.rating.is_some());
    assert_eq!(skill.rating.as_ref().unwrap().count, 1);
    assert_eq!(skill.rating.as_ref().unwrap().average, 4.0);
}

#[test]
fn test_cli_execute_rate_nonexistent() {
    let mut cli = MarketplaceCLI::new();

    let rating = Rating::new(5).unwrap();
    let result = cli.execute(Command::Rate {
        id: "non-existent".to_string(),
        rating,
    });
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), MarketplaceError::SkillNotFound(_)));
}

#[test]
fn test_cli_execute_review() {
    let mut cli = MarketplaceCLI::new();
    cli.registry_mut().register(create_skill("review-test", "Review Test", vec![], vec![])).unwrap();

    let rating = Rating::new(5).unwrap();
    let result = cli.execute(Command::Review {
        id: "review-test".to_string(),
        rating,
        comment: Some("Excellent skill!".to_string()),
        author: "bob".to_string(),
    });
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Added review"));

    // Verify review was added
    let skill = cli.registry().get("review-test").unwrap();
    assert!(skill.rating.is_some());
    let skill_rating = skill.rating.as_ref().unwrap();
    assert_eq!(skill_rating.count, 1);
    assert_eq!(skill_rating.reviews.len(), 1);
    assert_eq!(skill_rating.reviews[0].author, "bob");
    assert_eq!(skill_rating.reviews[0].comment, Some("Excellent skill!".to_string()));
}

#[test]
fn test_cli_execute_show() {
    let mut cli = MarketplaceCLI::new();
    cli.registry_mut().register(create_skill("show-test", "Show Test", vec![SkillCategory::CodeGeneration], vec!["test".to_string()])).unwrap();

    let result = cli.execute(Command::Show { id: "show-test".to_string() });
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Show Test"));
    assert!(output.contains("show-test"));
    assert!(output.contains("Code Generation"));
}

#[test]
fn test_cli_execute_top_rated() {
    let mut cli = MarketplaceCLI::new();

    // Add skills with different ratings
    let mut skill_a = create_skill("a", "Skill A", vec![], vec![]);
    let mut rating_a = SkillRating::new();
    rating_a.add_rating(Rating::new(5).unwrap());
    skill_a.rating = Some(rating_a);
    cli.registry_mut().register(skill_a).unwrap();

    let mut skill_b = create_skill("b", "Skill B", vec![], vec![]);
    let mut rating_b = SkillRating::new();
    rating_b.add_rating(Rating::new(3).unwrap());
    skill_b.rating = Some(rating_b);
    cli.registry_mut().register(skill_b).unwrap();

    let result = cli.execute(Command::TopRated { n: 2 });
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Skill A"));
    assert!(output.contains("Skill B"));
}

#[test]
fn test_cli_execute_help() {
    let mut cli = MarketplaceCLI::new();
    let result = cli.execute(Command::Help);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Nebula Marketplace CLI"));
    assert!(output.contains("Commands:"));
    assert!(output.contains("list"));
    assert!(output.contains("search"));
    assert!(output.contains("install"));
}

#[test]
fn test_cli_parse_category_variations() {
    // Various category name formats
    assert_eq!(
        nebula_marketplace::parse_category("code-generation").unwrap(),
        SkillCategory::CodeGeneration
    );
    assert_eq!(
        nebula_marketplace::parse_category("code_generation").unwrap(),
        SkillCategory::CodeGeneration
    );
    assert_eq!(
        nebula_marketplace::parse_category("code generation").unwrap(),
        SkillCategory::CodeGeneration
    );
    assert_eq!(
        nebula_marketplace::parse_category("CODEGENERATION").unwrap(),
        SkillCategory::CodeGeneration
    );
    assert_eq!(
        nebula_marketplace::parse_category("dataanalysis").unwrap(),
        SkillCategory::DataAnalysis
    );
    assert_eq!(
        nebula_marketplace::parse_category("devops").unwrap(),
        SkillCategory::DevOps
    );
    assert_eq!(
        nebula_marketplace::parse_category("dev ops").unwrap(),
        SkillCategory::DevOps
    );
}

#[test]
fn test_cli_workflow_with_parsing() {
    let mut cli = MarketplaceCLI::new();

    // Register some skills
    cli.registry_mut().register(create_skill("async-util", "Async Utility", vec![SkillCategory::CodeGeneration], vec!["async".to_string(), "rust".to_string()])).unwrap();
    cli.registry_mut().register(create_skill("data-proc", "Data Processor", vec![SkillCategory::DataAnalysis], vec!["data".to_string()])).unwrap();

    // Parse and execute: list
    let args = vec!["list".to_string()];
    let cmd = parse_command(&args).unwrap();
    let result = cli.execute(cmd);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("2 skill(s)"));

    // Parse and execute: search
    let args = vec!["search".to_string(), "async".to_string()];
    let cmd = parse_command(&args).unwrap();
    let result = cli.execute(cmd);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("Async Utility"));

    // Parse and execute: install
    let args = vec!["install".to_string(), "async-util".to_string()];
    let cmd = parse_command(&args).unwrap();
    let result = cli.execute(cmd);
    assert!(result.is_ok());
    assert!(cli.registry().get("async-util").unwrap().installed);

    // Parse and execute: rate
    let args = vec!["rate".to_string(), "async-util".to_string(), "5".to_string()];
    let cmd = parse_command(&args).unwrap();
    let result = cli.execute(cmd);
    assert!(result.is_ok());

    // Parse and execute: list installed
    let args = vec!["list".to_string(), "--installed".to_string()];
    let cmd = parse_command(&args).unwrap();
    let result = cli.execute(cmd);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("1 skill(s)"));
}
