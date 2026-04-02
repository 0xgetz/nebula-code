use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a Nebula Code project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Project name
    pub name: String,
    
    /// Project description
    pub description: String,
    
    /// Project version
    pub version: String,
    
    /// Root directory of the project
    pub root: PathBuf,
    
    /// Project type
    pub project_type: ProjectType,
    
    /// Installed skill cards
    pub skills: Vec<InstalledSkill>,
    
    /// Project configuration
    pub config: ProjectConfig,
}

/// Type of Nebula project
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectType {
    Web,
    Desktop,
    Cli,
    Library,
    Other,
}

/// An installed skill in the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledSkill {
    /// Skill card ID
    pub skill_id: String,
    
    /// Installed version
    pub version: String,
    
    /// Custom configuration for this installation
    pub config: serde_json::Value,
    
    /// Whether the skill is enabled
    pub enabled: bool,
}

/// Project configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Build configuration
    pub build: BuildConfig,
    
    /// Test configuration
    pub test: TestConfig,
    
    /// Deployment configuration
    pub deploy: Option<DeployConfig>,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Output directory
    pub output_dir: PathBuf,
    
    /// Entry point
    pub entry_point: Option<PathBuf>,
    
    /// Build optimization level
    pub optimize: bool,
}

/// Test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// Test directories
    pub test_dirs: Vec<PathBuf>,
    
    /// Coverage requirements
    pub coverage_threshold: Option<f32>,
}

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    /// Deployment target
    pub target: String,
    
    /// Pre-deploy hooks
    pub pre_deploy: Vec<String>,
    
    /// Post-deploy hooks
    pub post_deploy: Vec<String>,
}

impl Project {
    /// Create a new project
    pub fn new(name: impl Into<String>, root: PathBuf) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            version: "0.1.0".to_string(),
            root,
            project_type: ProjectType::Other,
            skills: vec![],
            config: ProjectConfig {
                build: BuildConfig {
                    output_dir: PathBuf::from("dist"),
                    entry_point: None,
                    optimize: false,
                },
                test: TestConfig {
                    test_dirs: vec![PathBuf::from("tests")],
                    coverage_threshold: Some(80.0),
                },
                deploy: None,
            },
        }
    }
}
