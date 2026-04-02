//! Nebula Core Library
//!
//! Provides fundamental types and functions for Nebula Code.

pub mod skill_card;
pub mod project;
pub mod error;

pub use skill_card::SkillCard;
pub use project::Project;
pub use error::NebulaError;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
