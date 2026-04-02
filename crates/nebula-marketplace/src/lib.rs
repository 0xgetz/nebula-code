//! Nebula Marketplace - Core types and registry for skill management
//!
//! This crate provides the foundational types and data structures for the Nebula
//! skill marketplace, including skill metadata, versioning, registry management,
//! discovery capabilities, and skill execution.
//!
//! # Modules
//!
//! - `types`: Core data structures (Skill, SkillMetadata, SkillManifest, etc.)
//! - `registry`: SkillRegistry for storing and managing skills
//! - `discovery`: Skill discovery and filtering capabilities
//! - `rating`: Skill rating and review system
//! - `execution`: Skill execution engine with generic executors
//!
//! # Example
//!
//! ```
//! use nebula_marketplace::types::{Skill, SkillMetadata, SkillManifest, SkillCategory};
//! use nebula_marketplace::registry::SkillRegistry;
//! use nebula_marketplace::discovery::SkillQuery;
//!
//! // Create a new registry
//! let mut registry = SkillRegistry::new();
//!
//! // Create a skill
//! let metadata = SkillMetadata::new(
//!     "example-skill".to_string(),
//!     "An example skill".to_string(),
//!     "author".to_string(),
//! )
//! .with_categories(vec![SkillCategory::CodeGeneration]);
//!
//! let manifest = SkillManifest::new("main.rs".to_string(), "rust".to_string());
//! let skill = Skill::new("example-1".to_string(), metadata, manifest);
//!
//! // Register the skill
//! registry.register(skill).unwrap();
//!
//! // Search for skills
//! let query = SkillQuery::new().with_name("example");
//! let results = registry.search_by_query(&query);
//! assert_eq!(results.len(), 1);
//! ```
//!
//! # Execution Engine
//!
//! The execution module provides a generic framework for running skills:
//!
//! ```
//! use nebula_marketplace::execution::{
//!     SkillExecutor, ExecutionContext, SkillInstance, ExecutionOutput,
//!     ExecutorRegistry, ExecutionStatus, SkillExecutorError, SkillInstanceConfig,
//! };
//! use std::fmt;
//!
//! // Define a simple executor
//! #[derive(Debug)]
//! struct EchoExecutor;
//!
//! impl SkillExecutor<String, String> for EchoExecutor {
//!     fn execute(&self, ctx: &ExecutionContext<String>) -> Result<ExecutionOutput<String>, SkillExecutorError> {
//!         Ok(ExecutionOutput::success(
//!             ExecutionStatus::Completed,
//!             ctx.input().clone(),
//!         ))
//!     }
//!
//!     fn validate(&self, ctx: &ExecutionContext<String>) -> Result<(), SkillExecutorError> {
//!         if ctx.input().is_empty() {
//!             return Err(SkillExecutorError::ValidationError("Input cannot be empty".to_string()));
//!         }
//!         Ok(())
//!     }
//!
//!     fn prepare(&self, _instance: &mut SkillInstance<String, String>) -> Result<(), SkillExecutorError> {
//!         Ok(())
//!     }
//!
//!     fn skill_type(&self) -> &str {
//!         "echo"
//!     }
//! }
//!
//! // Create and use an executor registry
//! let mut registry = ExecutorRegistry::<String, String>::new();
//! registry.register("echo".to_string(), Box::new(EchoExecutor)).unwrap();
//!
//! let ctx = ExecutionContext::new("Hello, World!".to_string());
//! let result = registry.execute("echo", &ctx).unwrap();
//! assert!(result.is_success());
//! assert_eq!(result.value(), Some(&"Hello, World!".to_string()));
//! ```

pub mod cli;
pub mod dependencies;
pub mod discovery;
pub mod execution;
pub mod persistence;
pub mod rating;
pub mod registry;
pub mod types;

// Re-export commonly used types
pub use cli::{parse_category, parse_command, Command, MarketplaceCLI};
pub use discovery::{SkillDiscovery, SkillFilter, SkillQuery};
pub use registry::SkillRegistry;
pub use types::{
    MarketplaceError, Result, Skill, SkillCategory, SkillManifest, SkillMetadata, SkillVersion,
};

// Re-export rating types
pub use rating::{Rating, RatingQuery, Review, SkillRatable, SkillRating};

// Re-export execution types
pub use execution::{
    ExecutionOutput, ExecutionState, ExecutionStatus, ExecutionContext, SkillExecutor,
    SkillExecutorError, SkillInstance, ExecutorRegistry, ExecutionMetadata,
    SkillInstanceConfig, ResourceLimits, ExecutorCapabilities, ExecutionStats,
    ExecutionRecord, RegistryMetadata,
};

// Re-export dependency types
pub use dependencies::{
    Dependency, DependencyError, DependencyGraph, DependencyNode, DependencyResolver,
    ResolutionResult, ResolvedSkill, SkillMetadata as DependencySkillMetadata,
    VersionConflict,
};

// Re-export persistence types
pub use persistence::{
    FileSkillStorage, PersistenceError, PersistenceResult, SkillIndex,
    SkillPersistence, SkillStorage, StorageConfig,
};
