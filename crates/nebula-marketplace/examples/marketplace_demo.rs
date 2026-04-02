//! Marketplace CLI Demo
//!
//! This example demonstrates how to use the nebula-marketplace CLI to manage skills.

use nebula_marketplace::{
    Command, MarketplaceCLI, Rating, Skill, SkillCategory, SkillManifest, SkillMetadata,
    SkillVersion,
};

fn main() {
    println!("=== Nebula Marketplace CLI Demo ===\n");

    // Create a new CLI with an empty registry
    let mut cli = MarketplaceCLI::new();

    // Add some sample skills to the registry
    setup_sample_skills(&mut cli);

    // Demonstrate listing all skills
    println!("1. Listing all skills:");
    let output = cli.execute(Command::List {
        category: None,
        installed: None,
    });
    println!("{}\n", output.unwrap());

    // Demonstrate searching skills
    println!("2. Searching for 'async' skills:");
    let output = cli.execute(Command::Search {
        query: "async".to_string(),
        tags: vec![],
        author: None,
    });
    println!("{}\n", output.unwrap());

    // Demonstrate listing by category
    println!("3. Listing skills in 'Code Generation' category:");
    let output = cli.execute(Command::List {
        category: Some(SkillCategory::CodeGeneration),
        installed: None,
    });
    println!("{}\n", output.unwrap());

    // Demonstrate showing skill details
    println!("4. Showing details for 'async-helper' skill:");
    let output = cli.execute(Command::Show {
        id: "async-helper".to_string(),
    });
    println!("{}\n", output.unwrap());

    // Demonstrate installing a skill
    println!("5. Installing 'async-helper' skill:");
    let output = cli.execute(Command::Install {
        id: "async-helper".to_string(),
        path: Some("/opt/skills/async-helper".to_string()),
    });
    println!("{}\n", output.unwrap());

    // Demonstrate rating a skill
    println!("6. Rating 'async-helper' skill 5 stars:");
    let rating = Rating::new(5).unwrap();
    let output = cli.execute(Command::Rate {
        id: "async-helper".to_string(),
        rating,
    });
    println!("{}\n", output.unwrap());

    // Add another rating
    println!("7. Rating 'async-helper' skill 4 stars:");
    let rating = Rating::new(4).unwrap();
    let output = cli.execute(Command::Rate {
        id: "async-helper".to_string(),
        rating,
    });
    println!("{}\n", output.unwrap());

    // Demonstrate adding a review
    println!("8. Adding a review to 'async-helper' skill:");
    let rating = Rating::new(5).unwrap();
    let output = cli.execute(Command::Review {
        id: "async-helper".to_string(),
        rating,
        comment: Some("Excellent skill for async programming!".to_string()),
        author: "alice".to_string(),
    });
    println!("{}\n", output.unwrap());

    // Show the updated skill details with ratings
    println!("9. Showing updated details for 'async-helper' (with ratings):");
    let output = cli.execute(Command::Show {
        id: "async-helper".to_string(),
    });
    println!("{}\n", output.unwrap());

    // Demonstrate top rated skills
    println!("10. Showing top 3 rated skills:");
    let output = cli.execute(Command::TopRated { n: 3 });
    println!("{}\n", output.unwrap());

    // Demonstrate uninstalling a skill
    println!("11. Uninstalling 'async-helper' skill:");
    let output = cli.execute(Command::Uninstall {
        id: "async-helper".to_string(),
    });
    println!("{}\n", output.unwrap());

    // Show help
    println!("12. Showing help:");
    let output = cli.execute(Command::Help);
    println!("{}\n", output.unwrap());

    println!("=== Demo Complete ===");
}

fn setup_sample_skills(cli: &mut MarketplaceCLI) {
    let registry = cli.registry_mut();

    // Skill 1: Async Helper
    let metadata1 = SkillMetadata::new(
        "Async Helper".to_string(),
        "A skill for handling async operations in Rust".to_string(),
        "alice".to_string(),
    )
    .with_categories(vec![SkillCategory::CodeGeneration, SkillCategory::Development])
    .with_tags(vec!["rust".to_string(), "async".to_string(), "tokio".to_string()])
    .with_version(SkillVersion::new(1, 2, 0));
    let manifest1 = SkillManifest::new("lib.rs".to_string(), "rust".to_string())
        .with_runtime("tokio".to_string())
        .with_permissions(vec!["read".to_string(), "write".to_string()]);
    let skill1 = Skill::new("async-helper".to_string(), metadata1, manifest1);
    registry.register(skill1).unwrap();

    // Skill 2: Data Processor
    let metadata2 = SkillMetadata::new(
        "Data Processor".to_string(),
        "Process and analyze large datasets efficiently".to_string(),
        "bob".to_string(),
    )
    .with_categories(vec![SkillCategory::DataAnalysis, SkillCategory::Automation])
    .with_tags(vec!["data".to_string(), "processing".to_string(), "analytics".to_string()])
    .with_version(SkillVersion::new(0, 9, 5));
    let manifest2 = SkillManifest::new("main.py".to_string(), "python".to_string())
        .with_runtime("python3".to_string());
    let skill2 = Skill::new("data-processor".to_string(), metadata2, manifest2);
    registry.register(skill2).unwrap();

    // Skill 3: API Generator
    let metadata3 = SkillMetadata::new(
        "API Generator".to_string(),
        "Generate REST APIs from database schemas".to_string(),
        "charlie".to_string(),
    )
    .with_categories(vec![SkillCategory::CodeGeneration, SkillCategory::Development])
    .with_tags(vec!["api".to_string(), "rest".to_string(), "generator".to_string()])
    .with_version(SkillVersion::new(2, 0, 0));
    let manifest3 = SkillManifest::new("index.js".to_string(), "javascript".to_string())
        .with_runtime("node".to_string());
    let skill3 = Skill::new("api-generator".to_string(), metadata3, manifest3);
    registry.register(skill3).unwrap();

    // Skill 4: Security Scanner
    let metadata4 = SkillMetadata::new(
        "Security Scanner".to_string(),
        "Scan code for security vulnerabilities".to_string(),
        "diana".to_string(),
    )
    .with_categories(vec![SkillCategory::Security, SkillCategory::Testing])
    .with_tags(vec!["security".to_string(), "scanning".to_string(), "vulnerability".to_string()])
    .with_version(SkillVersion::new(1, 0, 0));
    let manifest4 = SkillManifest::new("scanner.go".to_string(), "go".to_string());
    let skill4 = Skill::new("security-scanner".to_string(), metadata4, manifest4);
    registry.register(skill4).unwrap();

    // Skill 5: Documentation Generator
    let metadata5 = SkillMetadata::new(
        "Doc Generator".to_string(),
        "Automatically generate documentation from code".to_string(),
        "eve".to_string(),
    )
    .with_categories(vec![SkillCategory::Documentation, SkillCategory::Automation])
    .with_tags(vec!["docs".to_string(), "documentation".to_string(), "generator".to_string()])
    .with_version(SkillVersion::new(0, 5, 2));
    let manifest5 = SkillManifest::new("docgen.rs".to_string(), "rust".to_string());
    let skill5 = Skill::new("doc-generator".to_string(), metadata5, manifest5);
    registry.register(skill5).unwrap();

    // Install a couple of skills
    registry.mark_installed("data-processor", Some("/opt/skills/data-processor".to_string())).unwrap();
    registry.mark_installed("security-scanner", Some("/opt/skills/security-scanner".to_string())).unwrap();

    // Add some ratings to make top-rated interesting
    {
        let skill = registry.get_mut("data-processor").unwrap();
        skill.rating = Some(nebula_marketplace::SkillRating::new());
    }
    {
        let skill = registry.get_mut("data-processor").unwrap();
        if let Some(ref mut rating) = skill.rating {
            rating.add_rating(Rating::new(5).unwrap());
            rating.add_rating(Rating::new(4).unwrap());
            rating.add_rating(Rating::new(5).unwrap());
        }
    }

    {
        let skill = registry.get_mut("security-scanner").unwrap();
        skill.rating = Some(nebula_marketplace::SkillRating::new());
    }
    {
        let skill = registry.get_mut("security-scanner").unwrap();
        if let Some(ref mut rating) = skill.rating {
            rating.add_rating(Rating::new(4).unwrap());
            rating.add_rating(Rating::new(4).unwrap());
            rating.add_rating(Rating::new(3).unwrap());
        }
    }

    {
        let skill = registry.get_mut("api-generator").unwrap();
        skill.rating = Some(nebula_marketplace::SkillRating::new());
    }
    {
        let skill = registry.get_mut("api-generator").unwrap();
        if let Some(ref mut rating) = skill.rating {
            rating.add_rating(Rating::new(5).unwrap());
            rating.add_rating(Rating::new(5).unwrap());
        }
    }
}
