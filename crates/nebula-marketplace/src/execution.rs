//! Skill Execution Engine
//!
//! This module provides the core execution infrastructure for running skills in the
//! Nebula marketplace. It includes:
//!
//! - `SkillExecutor`: Trait defining the execution interface for skills
//! - `ExecutionContext`: Environment and state for skill execution
//! - `SkillInstance`: Represents a running skill with its configuration
//! - `ExecutionResult`: Output, status, and errors from skill execution
//! - `ExecutorRegistry`: Manages available executors for different skill types
//!
//! # Example
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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during skill execution
#[derive(Error, Debug)]
pub enum SkillExecutorError {
    /// Validation failed for the given input
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Preparation failed before execution
    #[error("Preparation error: {0}")]
    PreparationError(String),

    /// Execution failed
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Executor not found for the given skill type
    #[error("Executor not found for skill type: {0}")]
    ExecutorNotFound(String),

    /// Executor already registered for the given skill type
    #[error("Executor already registered for skill type: {0}")]
    ExecutorAlreadyRegistered(String),

    /// Timeout during execution
    #[error("Execution timed out after {0:?}")]
    TimeoutError(Duration),

    /// Invalid execution state
    #[error("Invalid execution state: {0}")]
    InvalidState(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Resource not available
    #[error("Resource not available: {0}")]
    ResourceError(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionError(String),

    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Result type for execution operations
pub type ExecutionResult<T> = std::result::Result<T, SkillExecutorError>;

/// Status of a skill execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// Execution has not started yet
    Pending,
    /// Execution is currently running
    Running,
    /// Execution completed successfully
    Completed,
    /// Execution failed with an error
    Failed,
    /// Execution was cancelled by user
    Cancelled,
    /// Execution timed out
    TimedOut,
    /// Execution was skipped due to preconditions
    Skipped,
}

impl fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionStatus::Pending => write!(f, "pending"),
            ExecutionStatus::Running => write!(f, "running"),
            ExecutionStatus::Completed => write!(f, "completed"),
            ExecutionStatus::Failed => write!(f, "failed"),
            ExecutionStatus::Cancelled => write!(f, "cancelled"),
            ExecutionStatus::TimedOut => write!(f, "timed out"),
            ExecutionStatus::Skipped => write!(f, "skipped"),
        }
    }
}

/// Execution state tracking for a skill instance
#[derive(Debug, Clone)]
pub struct ExecutionState {
    /// Current status of execution
    pub status: ExecutionStatus,
    /// When execution started (as ISO 8601 string for serializability)
    pub started_at: Option<String>,
    /// Duration of execution in milliseconds (if completed)
    pub duration_ms: Option<u64>,
    /// Number of retries attempted
    pub retry_count: u32,
    /// Maximum number of retries allowed
    pub max_retries: u32,
    /// Current progress (0.0 to 1.0)
    pub progress: f64,
    /// Status message or description
    pub message: Option<String>,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self {
            status: ExecutionStatus::Pending,
            started_at: None,
            duration_ms: None,
            retry_count: 0,
            max_retries: 3,
            progress: 0.0,
            message: None,
        }
    }
}

impl ExecutionState {
    /// Create a new pending execution state
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark execution as started
    pub fn start(&mut self) {
        self.status = ExecutionStatus::Running;
        self.started_at = Some(chrono::Utc::now().to_rfc3339());
        self.progress = 0.0;
    }

    /// Mark execution as completed
    pub fn complete(&mut self) {
        self.status = ExecutionStatus::Completed;
        self.progress = 1.0;
        // Calculate duration from started_at if available
        if let Some(start_str) = &self.started_at {
            if let Ok(start_time) = chrono::DateTime::parse_from_rfc3339(start_str) {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(start_time);
                self.duration_ms = Some(duration.num_milliseconds() as u64);
            }
        }
    }

    /// Mark execution as failed
    pub fn fail(&mut self, error: String) {
        self.status = ExecutionStatus::Failed;
        self.message = Some(error);
        // Calculate duration from started_at if available
        if let Some(start_str) = &self.started_at {
            if let Ok(start_time) = chrono::DateTime::parse_from_rfc3339(start_str) {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(start_time);
                self.duration_ms = Some(duration.num_milliseconds() as u64);
            }
        }
    }

    /// Update progress
    pub fn set_progress(&mut self, progress: f64) {
        self.progress = progress.clamp(0.0, 1.0);
    }

    /// Increment retry count
    pub fn retry(&mut self) {
        self.retry_count += 1;
        self.started_at = None;
        self.duration_ms = None;
        self.status = ExecutionStatus::Pending;
    }

    /// Check if can retry
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Check if execution is terminal
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Completed
                | ExecutionStatus::Failed
                | ExecutionStatus::Cancelled
                | ExecutionStatus::TimedOut
                | ExecutionStatus::Skipped
        )
    }
}

/// Context for skill execution containing environment and parameters
#[derive(Debug, Clone)]
pub struct ExecutionContext<I> {
    /// Input parameters for the skill
    input: I,
    /// Environment variables available during execution
    env_vars: HashMap<String, String>,
    /// Configuration parameters
    config: HashMap<String, serde_json::Value>,
    /// Execution timeout
    timeout: Option<Duration>,
    /// Execution mode (e.g., "debug", "production")
    mode: String,
    /// Unique execution ID
    execution_id: String,
    /// Parent execution ID (for nested executions)
    parent_id: Option<String>,
}

impl<I> ExecutionContext<I> {
    /// Create a new execution context with the given input
    pub fn new(input: I) -> Self {
        Self {
            input,
            env_vars: HashMap::new(),
            config: HashMap::new(),
            timeout: None,
            mode: "production".to_string(),
            execution_id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
        }
    }

    /// Create a new execution context with input and environment variables
    pub fn with_env(input: I, env_vars: HashMap<String, String>) -> Self {
        Self {
            input,
            env_vars,
            config: HashMap::new(),
            timeout: None,
            mode: "production".to_string(),
            execution_id: uuid::Uuid::new_v4().to_string(),
            parent_id: None,
        }
    }

    /// Get the input
    pub fn input(&self) -> &I {
        &self.input
    }

    /// Get mutable input
    pub fn input_mut(&mut self) -> &mut I {
        &mut self.input
    }

    /// Get an environment variable
    pub fn env(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
    }

    /// Set an environment variable
    pub fn set_env(&mut self, key: String, value: String) -> &mut Self {
        self.env_vars.insert(key, value);
        self
    }

    /// Get all environment variables
    pub fn env_vars(&self) -> &HashMap<String, String> {
        &self.env_vars
    }

    /// Get a configuration value
    pub fn config(&self, key: &str) -> Option<&serde_json::Value> {
        self.config.get(key)
    }

    /// Set a configuration value
    pub fn set_config(&mut self, key: String, value: serde_json::Value) -> &mut Self {
        self.config.insert(key, value);
        self
    }

    /// Get all configuration values
    pub fn config_map(&self) -> &HashMap<String, serde_json::Value> {
        &self.config
    }

    /// Set execution timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Get timeout
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Set execution mode
    pub fn with_mode(mut self, mode: String) -> Self {
        self.mode = mode;
        self
    }

    /// Get execution mode
    pub fn mode(&self) -> &str {
        &self.mode
    }

    /// Get execution ID
    pub fn execution_id(&self) -> &str {
        &self.execution_id
    }

    /// Set parent execution ID
    pub fn with_parent_id(mut self, parent_id: String) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Get parent execution ID
    pub fn parent_id(&self) -> Option<&str> {
        self.parent_id.as_deref()
    }

    /// Check if debug mode is enabled
    pub fn is_debug(&self) -> bool {
        self.mode == "debug"
    }
}

/// Metadata about an execution
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionMetadata {
    /// Duration of execution in milliseconds
    pub duration_ms: u64,
    /// Memory used (in bytes)
    pub memory_used: Option<u64>,
    /// CPU time used (in nanoseconds)
    pub cpu_time_ns: Option<u64>,
    /// Number of operations performed
    pub operations_count: u64,
    /// Additional custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

/// Result of a skill execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOutput<O> {
    /// The output value from execution
    pub value: Option<O>,
    /// Execution status
    pub status: ExecutionStatus,
    /// Error message if failed
    pub error: Option<String>,
    /// Detailed error information
    pub error_details: Option<serde_json::Value>,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
}

impl<O> ExecutionOutput<O> {
    /// Create a successful execution result
    pub fn success(status: ExecutionStatus, value: O) -> Self {
        Self {
            value: Some(value),
            status,
            error: None,
            error_details: None,
            metadata: ExecutionMetadata::default(),
        }
    }

    /// Create a failed execution result
    pub fn failure(status: ExecutionStatus, error: String) -> Self {
        Self {
            value: None,
            status,
            error: Some(error),
            error_details: None,
            metadata: ExecutionMetadata::default(),
        }
    }

    /// Create a failed execution result with details
    pub fn failure_with_details(
        status: ExecutionStatus,
        error: String,
        details: serde_json::Value,
    ) -> Self {
        Self {
            value: None,
            status,
            error: Some(error),
            error_details: Some(details),
            metadata: ExecutionMetadata::default(),
        }
    }

    /// Check if execution was successful
    pub fn is_success(&self) -> bool {
        self.status == ExecutionStatus::Completed
    }

    /// Check if execution failed
    pub fn is_failure(&self) -> bool {
        matches!(
            self.status,
            ExecutionStatus::Failed | ExecutionStatus::TimedOut
        )
    }

    /// Get the value if successful, None otherwise
    pub fn value(&self) -> Option<&O> {
        self.value.as_ref()
    }

    /// Take the value if successful, None otherwise
    pub fn into_value(self) -> Option<O> {
        self.value
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: ExecutionMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set duration from Duration
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.metadata.duration_ms = duration.as_millis() as u64;
        self
    }
}

/// Configuration for a skill instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstanceConfig {
    /// Name of the skill
    pub name: String,
    /// Version of the skill
    pub version: String,
    /// Description
    pub description: Option<String>,
    /// Author
    pub author: Option<String>,
    /// Entry point or command to execute
    pub entry_point: Option<String>,
    /// Working directory
    pub working_dir: Option<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Configuration parameters
    pub parameters: HashMap<String, serde_json::Value>,
    /// Resource limits
    pub resources: ResourceLimits,
}

/// Resource limits for skill execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in bytes
    pub max_memory: Option<u64>,
    /// Maximum CPU percentage
    pub max_cpu: Option<f64>,
    /// Maximum execution time in seconds
    pub max_time: Option<u64>,
    /// Maximum number of processes
    pub max_processes: Option<u32>,
    /// Maximum number of open files
    pub max_open_files: Option<u32>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: None,
            max_cpu: None,
            max_time: None,
            max_processes: None,
            max_open_files: None,
        }
    }
}

impl Default for SkillInstanceConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            version: "0.1.0".to_string(),
            description: None,
            author: None,
            entry_point: None,
            working_dir: None,
            env: HashMap::new(),
            parameters: HashMap::new(),
            resources: ResourceLimits::default(),
        }
    }
}

/// A running skill instance with its configuration and state
#[derive(Debug)]
pub struct SkillInstance<I, O> {
    /// Unique identifier for this instance
    pub id: String,
    /// Skill type/name
    pub skill_type: String,
    /// Input type marker
    _input_marker: PhantomData<I>,
    /// Output type marker
    _output_marker: PhantomData<O>,
    /// Current execution state
    pub state: ExecutionState,
    /// Configuration for this instance
    pub config: SkillInstanceConfig,
    /// Dependencies that must be satisfied
    pub dependencies: Vec<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Priority for execution ordering
    pub priority: u32,
    /// Whether this instance is enabled
    pub enabled: bool,
}

impl<I, O> SkillInstance<I, O> {
    /// Create a new skill instance
    pub fn new(id: String, skill_type: String, config: SkillInstanceConfig) -> Self {
        Self {
            id,
            skill_type,
            _input_marker: PhantomData,
            _output_marker: PhantomData,
            state: ExecutionState::default(),
            config,
            dependencies: Vec::new(),
            tags: Vec::new(),
            priority: 0,
            enabled: true,
        }
    }

    /// Create a new skill instance with input/output types
    pub fn with_types<T, U>(
        id: String,
        skill_type: String,
        config: SkillInstanceConfig,
    ) -> SkillInstance<T, U> {
        SkillInstance {
            id,
            skill_type,
            _input_marker: PhantomData,
            _output_marker: PhantomData,
            state: ExecutionState::default(),
            config,
            dependencies: Vec::new(),
            tags: Vec::new(),
            priority: 0,
            enabled: true,
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dependency: String) -> &mut Self {
        self.dependencies.push(dependency);
        self
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) -> &mut Self {
        self.tags.push(tag);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    /// Set enabled state
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if instance is ready to execute
    pub fn is_ready(&self) -> bool {
        self.enabled
            && !self.state.is_terminal()
            && self.dependencies.iter().all(|_dep| {
                // In a real implementation, this would check if the dependency is satisfied
                true
            })
    }

    /// Get current status
    pub fn status(&self) -> ExecutionStatus {
        self.state.status
    }
}

/// Capabilities of an executor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorCapabilities {
    /// Whether the executor supports async execution
    pub supports_async: bool,
    /// Whether the executor supports streaming output
    pub supports_streaming: bool,
    /// Whether the executor supports cancellation
    pub supports_cancellation: bool,
    /// Whether the executor supports retries
    pub supports_retries: bool,
    /// Whether the executor supports progress reporting
    pub supports_progress: bool,
    /// Maximum concurrent executions
    pub max_concurrent: u32,
    /// Supported input formats
    pub input_formats: Vec<String>,
    /// Supported output formats
    pub output_formats: Vec<String>,
}

impl Default for ExecutorCapabilities {
    fn default() -> Self {
        Self {
            supports_async: false,
            supports_streaming: false,
            supports_cancellation: true,
            supports_retries: true,
            supports_progress: true,
            max_concurrent: 1,
            input_formats: vec!["json".to_string()],
            output_formats: vec!["json".to_string()],
        }
    }
}

/// Trait for executing skills
///
/// This trait defines the interface that all skill executors must implement.
/// It is generic over input type I and output type O.
pub trait SkillExecutor<I, O>: Send + Sync + fmt::Debug {
    /// Execute the skill with the given context
    ///
    /// This is the main entry point for skill execution. The executor should:
    /// 1. Validate the input (or rely on separate validate call)
    /// 2. Perform the skill's logic
    /// 3. Return the result
    fn execute(&self, ctx: &ExecutionContext<I>) -> ExecutionResult<ExecutionOutput<O>>;

    /// Validate the input and configuration before execution
    ///
    /// This method should check:
    /// - Input format and constraints
    /// - Required configuration parameters
    /// - Resource availability
    /// - Permissions
    fn validate(&self, ctx: &ExecutionContext<I>) -> ExecutionResult<()>;

    /// Prepare the skill instance for execution
    ///
    /// This method is called before execute() and can be used for:
    /// - Loading resources
    /// - Initializing state
    /// - Setting up environment
    /// - Pre-compiling or caching
    fn prepare(&self, instance: &mut SkillInstance<I, O>) -> ExecutionResult<()>;

    /// Get the skill type this executor handles
    fn skill_type(&self) -> &str;

    /// Get executor capabilities
    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities::default()
    }

    /// Called after execution for cleanup
    fn cleanup(&self, _instance: &SkillInstance<I, O>) -> ExecutionResult<()> {
        Ok(())
    }

    /// Get executor version
    fn version(&self) -> &str {
        "0.1.0"
    }
}

/// Record of an execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// Execution ID
    pub execution_id: String,
    /// Skill type
    pub skill_type: String,
    /// Instance ID
    pub instance_id: Option<String>,
    /// Status
    pub status: ExecutionStatus,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Timestamp
    pub timestamp: String,
    /// Error message if any
    pub error: Option<String>,
}

/// Metadata for the executor registry
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistryMetadata {
    /// Registry name
    pub name: Option<String>,
    /// Registry version
    pub version: String,
    /// Description
    pub description: Option<String>,
    /// Total executions
    pub total_executions: u64,
    /// Successful executions
    pub successful_executions: u64,
    /// Failed executions
    pub failed_executions: u64,
}

/// Statistics about executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Total number of executions
    pub total_executions: u64,
    /// Number of successful executions
    pub successful_executions: u64,
    /// Number of failed executions
    pub failed_executions: u64,
    /// Success rate as percentage
    pub success_rate: f64,
    /// Average execution duration in milliseconds
    pub average_duration_ms: f64,
}

/// Registry for managing skill executors
#[derive(Debug)]
pub struct ExecutorRegistry<I, O> {
    /// Map of skill type to executor
    executors: HashMap<String, Box<dyn SkillExecutor<I, O>>>,
    /// Default executor for unknown skill types
    default_executor: Option<Box<dyn SkillExecutor<I, O>>>,
    /// Execution history
    history: Vec<ExecutionRecord>,
    /// Registry metadata
    metadata: RegistryMetadata,
}

impl<I, O> Default for ExecutorRegistry<I, O> {
    fn default() -> Self {
        Self::new()
    }
}

impl<I, O> ExecutorRegistry<I, O> {
    /// Create a new empty executor registry
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
            default_executor: None,
            history: Vec::new(),
            metadata: RegistryMetadata {
                name: None,
                version: "0.1.0".to_string(),
                description: None,
                total_executions: 0,
                successful_executions: 0,
                failed_executions: 0,
            },
        }
    }

    /// Register an executor for a skill type
    pub fn register(
        &mut self,
        skill_type: String,
        executor: Box<dyn SkillExecutor<I, O>>,
    ) -> ExecutionResult<()> {
        if self.executors.contains_key(&skill_type) {
            return Err(SkillExecutorError::ExecutorAlreadyRegistered(skill_type));
        }
        self.executors.insert(skill_type, executor);
        Ok(())
    }

    /// Register or replace an executor for a skill type
    pub fn register_or_replace(
        &mut self,
        skill_type: String,
        executor: Box<dyn SkillExecutor<I, O>>,
    ) {
        self.executors.insert(skill_type, executor);
    }

    /// Set the default executor for unknown skill types
    pub fn set_default(&mut self, executor: Box<dyn SkillExecutor<I, O>>) {
        self.default_executor = Some(executor);
    }

    /// Get an executor for a skill type
    pub fn get(&self, skill_type: &str) -> Option<&dyn SkillExecutor<I, O>> {
        self.executors
            .get(skill_type)
            .map(|e| e.as_ref())
            .or(self.default_executor.as_ref().map(|e| e.as_ref()))
    }

    /// Get a mutable reference to an executor
    pub fn get_mut(&mut self, skill_type: &str) -> Option<&mut Box<dyn SkillExecutor<I, O>>> {
        self.executors.get_mut(skill_type)
    }

    /// Unregister an executor
    pub fn unregister(&mut self, skill_type: &str) -> Option<Box<dyn SkillExecutor<I, O>>> {
        self.executors.remove(skill_type)
    }

    /// Check if an executor is registered
    pub fn has_executor(&self, skill_type: &str) -> bool {
        self.executors.contains_key(skill_type)
    }

    /// Get all registered skill types
    pub fn registered_types(&self) -> Vec<&str> {
        self.executors.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of registered executors
    pub fn len(&self) -> usize {
        self.executors.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.executors.is_empty()
    }

    /// Execute a skill by type (does not update history - use for stateless execution)
    pub fn execute(
        &self,
        skill_type: &str,
        ctx: &ExecutionContext<I>,
    ) -> ExecutionResult<ExecutionOutput<O>> {
        let executor = self
            .get(skill_type)
            .ok_or_else(|| SkillExecutorError::ExecutorNotFound(skill_type.to_string()))?;

        // Validate first
        executor.validate(ctx)?;

        // Execute
        executor.execute(ctx)
    }

    /// Execute with a specific instance (updates instance state and history)
    pub fn execute_instance(
        &mut self,
        instance: &mut SkillInstance<I, O>,
        ctx: &ExecutionContext<I>,
    ) -> ExecutionResult<ExecutionOutput<O>> {
        // Check if enabled
        if !instance.enabled {
            return Err(SkillExecutorError::InvalidState(format!(
                "Instance {} is disabled",
                instance.id
            )));
        }

        // Get executor skill type
        let skill_type = instance.skill_type.clone();

        // Get executor
        let executor = self
            .get(&skill_type)
            .ok_or_else(|| SkillExecutorError::ExecutorNotFound(skill_type.clone()))?;

        // Prepare
        executor.prepare(instance)?;

        // Start execution
        instance.state.start();

        // Execute (we need to drop the executor borrow before recording)
        let result = executor.execute(ctx);

        // Update state and record based on result
        let (status, error_msg) = match &result {
            Ok(output) => {
                instance.state.complete();
                (output.status, None)
            }
            Err(e) => {
                instance.state.fail(e.to_string());
                (ExecutionStatus::Failed, Some(e.to_string()))
            }
        };

        // Record in history (separate mutable borrow)
        self.record_execution(
            skill_type,
            Some(instance.id.clone()),
            status,
            error_msg,
        );

        // Cleanup
        let executor = self
            .get(&instance.skill_type)
            .ok_or_else(|| SkillExecutorError::ExecutorNotFound(instance.skill_type.clone()))?;
        let _ = executor.cleanup(instance);

        result
    }

    /// Record an execution in history
    fn record_execution(
        &mut self,
        skill_type: String,
        instance_id: Option<String>,
        status: ExecutionStatus,
        error: Option<String>,
    ) {
        let duration_ms = 0; // Will be updated if instance state has duration
        let timestamp = chrono::Utc::now().to_rfc3339();

        self.metadata.total_executions += 1;
        if status == ExecutionStatus::Completed {
            self.metadata.successful_executions += 1;
        } else {
            self.metadata.failed_executions += 1;
        }

        self.history.push(ExecutionRecord {
            execution_id: uuid::Uuid::new_v4().to_string(),
            skill_type,
            instance_id,
            status,
            duration_ms,
            timestamp,
            error,
        });

        // Keep only last 1000 records
        if self.history.len() > 1000 {
            self.history.remove(0);
        }
    }

    /// Get execution history
    pub fn history(&self) -> &[ExecutionRecord] {
        &self.history
    }

    /// Get registry metadata
    pub fn metadata(&self) -> &RegistryMetadata {
        &self.metadata
    }

    /// Set registry name
    pub fn with_name(mut self, name: String) -> Self {
        self.metadata.name = Some(name);
        self
    }

    /// Set registry description
    pub fn with_description(mut self, description: String) -> Self {
        self.metadata.description = Some(description);
        self
    }

    /// Get statistics
    pub fn stats(&self) -> ExecutionStats {
        let total = self.metadata.total_executions;
        let success = self.metadata.successful_executions;
        let failed = self.metadata.failed_executions;

        ExecutionStats {
            total_executions: total,
            successful_executions: success,
            failed_executions: failed,
            success_rate: if total > 0 {
                (success as f64 / total as f64) * 100.0
            } else {
                0.0
            },
            average_duration_ms: self.calculate_average_duration(),
        }
    }

    /// Calculate average execution duration
    fn calculate_average_duration(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        let total: u64 = self.history.iter().map(|r| r.duration_ms).sum();
        total as f64 / self.history.len() as f64
    }

    /// Clear execution history
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.metadata.total_executions = 0;
        self.metadata.successful_executions = 0;
        self.metadata.failed_executions = 0;
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test executor
    #[derive(Debug)]
    struct TestExecutor;

    impl SkillExecutor<String, String> for TestExecutor {
        fn execute(
            &self,
            ctx: &ExecutionContext<String>,
        ) -> ExecutionResult<ExecutionOutput<String>> {
            Ok(ExecutionOutput::success(
                ExecutionStatus::Completed,
                format!("Processed: {}", ctx.input()),
            ))
        }

        fn validate(&self, ctx: &ExecutionContext<String>) -> ExecutionResult<()> {
            if ctx.input().is_empty() {
                return Err(SkillExecutorError::ValidationError(
                    "Input cannot be empty".to_string(),
                ));
            }
            Ok(())
        }

        fn prepare(
            &self,
            _instance: &mut SkillInstance<String, String>,
        ) -> ExecutionResult<()> {
            Ok(())
        }

        fn skill_type(&self) -> &str {
            "test"
        }
    }

    // Test executor that fails
    #[derive(Debug)]
    struct FailingExecutor;

    impl SkillExecutor<String, String> for FailingExecutor {
        fn execute(
            &self,
            _ctx: &ExecutionContext<String>,
        ) -> ExecutionResult<ExecutionOutput<String>> {
            Err(SkillExecutorError::ExecutionError(
                "Intentional failure".to_string(),
            ))
        }

        fn validate(&self, _ctx: &ExecutionContext<String>) -> ExecutionResult<()> {
            Ok(())
        }

        fn prepare(
            &self,
            _instance: &mut SkillInstance<String, String>,
        ) -> ExecutionResult<()> {
            Ok(())
        }

        fn skill_type(&self) -> &str {
            "failing"
        }
    }

    #[test]
    fn test_execution_status_display() {
        assert_eq!(ExecutionStatus::Pending.to_string(), "pending");
        assert_eq!(ExecutionStatus::Running.to_string(), "running");
        assert_eq!(ExecutionStatus::Completed.to_string(), "completed");
        assert_eq!(ExecutionStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_execution_state_transitions() {
        let mut state = ExecutionState::new();
        assert_eq!(state.status, ExecutionStatus::Pending);
        assert!(!state.is_terminal());

        state.start();
        assert_eq!(state.status, ExecutionStatus::Running);
        assert!(!state.is_terminal());

        state.complete();
        assert_eq!(state.status, ExecutionStatus::Completed);
        assert!(state.is_terminal());
        assert!(state.duration_ms.is_some());
    }

    #[test]
    fn test_execution_state_retry() {
        let mut state = ExecutionState::new();
        state.start();
        state.fail("error".to_string());
        assert!(state.can_retry());

        state.retry();
        assert_eq!(state.retry_count, 1);
        assert_eq!(state.status, ExecutionStatus::Pending);

        state.max_retries = 1;
        assert!(!state.can_retry());
    }

    #[test]
    fn test_execution_context() {
        let mut env = HashMap::new();
        env.insert("KEY".to_string(), "value".to_string());

        let ctx = ExecutionContext::with_env("input".to_string(), env.clone());
        assert_eq!(ctx.input(), "input");
        assert_eq!(ctx.env("KEY"), Some(&"value".to_string()));
        assert_eq!(ctx.env_vars(), &env);
        assert!(!ctx.is_debug());

        let ctx = ctx.with_mode("debug".to_string());
        assert!(ctx.is_debug());
    }

    #[test]
    fn test_execution_output() {
        let output = ExecutionOutput::success(ExecutionStatus::Completed, "result".to_string());
        assert!(output.is_success());
        assert_eq!(output.value(), Some(&"result".to_string()));

        let failure: ExecutionOutput<String> = ExecutionOutput::failure(ExecutionStatus::Failed, "error".to_string());
        assert!(failure.is_failure());
        assert!(failure.value().is_none());
    }

    #[test]
    fn test_executor_registry_basic() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        assert!(registry.is_empty());

        registry
            .register("test".to_string(), Box::new(TestExecutor))
            .unwrap();
        assert!(!registry.is_empty());
        assert!(registry.has_executor("test"));
        assert!(!registry.has_executor("unknown"));

        let types = registry.registered_types();
        assert_eq!(types.len(), 1);
        assert!(types.contains(&"test"));
    }

    #[test]
    fn test_executor_registry_execute() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        registry
            .register("test".to_string(), Box::new(TestExecutor))
            .unwrap();

        let ctx = ExecutionContext::new("hello".to_string());
        let result = registry.execute("test", &ctx).unwrap();
        assert!(result.is_success());
        assert_eq!(result.value(), Some(&"Processed: hello".to_string()));
    }

    #[test]
    fn test_executor_registry_execute_validation_failure() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        registry
            .register("test".to_string(), Box::new(TestExecutor))
            .unwrap();

        let ctx = ExecutionContext::new("".to_string());
        let result = registry.execute("test", &ctx);
        assert!(result.is_err());
        match result {
            Err(SkillExecutorError::ValidationError(msg)) => {
                assert_eq!(msg, "Input cannot be empty");
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[test]
    fn test_executor_registry_execute_not_found() {
        let registry = ExecutorRegistry::<String, String>::new();
        let ctx = ExecutionContext::new("hello".to_string());
        let result = registry.execute("unknown", &ctx);
        assert!(result.is_err());
        match result {
            Err(SkillExecutorError::ExecutorNotFound(type_name)) => {
                assert_eq!(type_name, "unknown");
            }
            _ => panic!("Expected ExecutorNotFound"),
        }
    }

    #[test]
    fn test_executor_registry_default_executor() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        registry.set_default(Box::new(TestExecutor));

        let ctx = ExecutionContext::new("hello".to_string());
        let result = registry.execute("any_type", &ctx).unwrap();
        assert!(result.is_success());
    }

    #[test]
    fn test_executor_registry_instance_execution() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        registry
            .register("test".to_string(), Box::new(TestExecutor))
            .unwrap();

        let config = SkillInstanceConfig {
            name: "Test Instance".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        };
        let mut instance = SkillInstance::new("inst-1".to_string(), "test".to_string(), config);

        let ctx = ExecutionContext::new("world".to_string());
        let result = registry.execute_instance(&mut instance, &ctx).unwrap();
        assert!(result.is_success());
        assert_eq!(instance.status(), ExecutionStatus::Completed);
    }

    #[test]
    fn test_executor_registry_stats() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        registry
            .register("test".to_string(), Box::new(TestExecutor))
            .unwrap();
        registry
            .register("failing".to_string(), Box::new(FailingExecutor))
            .unwrap();

        let ctx = ExecutionContext::new("hello".to_string());
        let _ = registry.execute("test", &ctx);
        let _ = registry.execute("failing", &ctx);

        let stats = registry.stats();
        assert_eq!(stats.total_executions, 0); // execute() doesn't record
        assert_eq!(stats.successful_executions, 0);
    }

    #[test]
    fn test_executor_registry_instance_stats() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        registry
            .register("test".to_string(), Box::new(TestExecutor))
            .unwrap();
        registry
            .register("failing".to_string(), Box::new(FailingExecutor))
            .unwrap();

        let config1 = SkillInstanceConfig {
            name: "Test 1".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        };
        let mut instance1 = SkillInstance::new("inst-1".to_string(), "test".to_string(), config1);

        let config2 = SkillInstanceConfig {
            name: "Test 2".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        };
        let mut instance2 = SkillInstance::new("inst-2".to_string(), "failing".to_string(), config2);

        let ctx = ExecutionContext::new("hello".to_string());
        let _ = registry.execute_instance(&mut instance1, &ctx);
        let _ = registry.execute_instance(&mut instance2, &ctx);

        let stats = registry.stats();
        assert_eq!(stats.total_executions, 2);
        assert_eq!(stats.successful_executions, 1);
        assert_eq!(stats.failed_executions, 1);
        assert!((stats.success_rate - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_skill_instance() {
        let config = SkillInstanceConfig {
            name: "My Skill".to_string(),
            version: "2.0.0".to_string(),
            description: Some("A test skill".to_string()),
            ..Default::default()
        };
        let mut instance = SkillInstance::<String, String>::new(
            "id-1".to_string(),
            "my_skill".to_string(),
            config,
        );

        instance
            .add_dependency("dep1".to_string())
            .add_tag("tag1".to_string())
            .add_tag("tag2".to_string());

        assert_eq!(instance.dependencies.len(), 1);
        assert_eq!(instance.tags.len(), 2);
        assert!(instance.is_ready());

        instance.set_enabled(false);
        assert!(!instance.is_ready());
    }

    #[test]
    fn test_resource_limits() {
        let limits = ResourceLimits {
            max_memory: Some(1024 * 1024 * 100), // 100MB
            max_cpu: Some(80.0),
            max_time: Some(30),
            ..Default::default()
        };

        assert_eq!(limits.max_memory, Some(104857600));
        assert_eq!(limits.max_cpu, Some(80.0));
        assert_eq!(limits.max_time, Some(30));
    }

    #[test]
    fn test_executor_capabilities() {
        let caps = ExecutorCapabilities::default();
        assert!(caps.supports_cancellation);
        assert!(caps.supports_retries);
        assert!(caps.supports_progress);
        assert!(!caps.supports_async);
        assert!(!caps.supports_streaming);
        assert_eq!(caps.max_concurrent, 1);
    }

    #[test]
    fn test_registry_metadata() {
        let registry = ExecutorRegistry::<String, String>::new()
            .with_name("Test Registry".to_string())
            .with_description("A test registry".to_string());

        assert_eq!(registry.metadata().name, Some("Test Registry".to_string()));
        assert_eq!(
            registry.metadata().description,
            Some("A test registry".to_string())
        );
        assert_eq!(registry.metadata().version, "0.1.0");
    }

    #[test]
    fn test_registry_clear_history() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        registry
            .register("test".to_string(), Box::new(TestExecutor))
            .unwrap();

        let config = SkillInstanceConfig {
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        };
        let mut instance = SkillInstance::new("inst-1".to_string(), "test".to_string(), config);

        let ctx = ExecutionContext::new("a".to_string());
        let _ = registry.execute_instance(&mut instance, &ctx);

        // Create another instance for second execution
        let config2 = SkillInstanceConfig {
            name: "Test2".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        };
        let mut instance2 = SkillInstance::new("inst-2".to_string(), "test".to_string(), config2);
        let _ = registry.execute_instance(&mut instance2, &ctx);

        assert_eq!(registry.history().len(), 2);
        assert_eq!(registry.stats().total_executions, 2);

        registry.clear_history();
        assert!(registry.history().is_empty());
        assert_eq!(registry.stats().total_executions, 0);
    }

    #[test]
    fn test_execution_output_with_metadata() {
        let mut metadata = ExecutionMetadata::default();
        metadata.duration_ms = 1234;
        metadata.operations_count = 42;

        let output = ExecutionOutput::success(ExecutionStatus::Completed, "result".to_string())
            .with_metadata(metadata.clone());

        assert_eq!(output.metadata.duration_ms, 1234);
        assert_eq!(output.metadata.operations_count, 42);
    }

    #[test]
    fn test_executor_registry_unregister() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        registry
            .register("test".to_string(), Box::new(TestExecutor))
            .unwrap();

        assert!(registry.has_executor("test"));
        let removed = registry.unregister("test");
        assert!(removed.is_some());
        assert!(!registry.has_executor("test"));
        assert!(registry.is_empty());
    }

    #[test]
    fn test_executor_registry_register_duplicate() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        registry
            .register("test".to_string(), Box::new(TestExecutor))
            .unwrap();

        let result = registry.register("test".to_string(), Box::new(TestExecutor));
        assert!(result.is_err());
        match result {
            Err(SkillExecutorError::ExecutorAlreadyRegistered(type_name)) => {
                assert_eq!(type_name, "test");
            }
            _ => panic!("Expected ExecutorAlreadyRegistered"),
        }
    }

    #[test]
    fn test_executor_registry_replace() {
        let mut registry = ExecutorRegistry::<String, String>::new();
        registry.register_or_replace("test".to_string(), Box::new(TestExecutor));
        registry.register_or_replace("test".to_string(), Box::new(FailingExecutor));

        // Should now use FailingExecutor
        let ctx = ExecutionContext::new("hello".to_string());
        let result = registry.execute("test", &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_skill_instance_config() {
        let mut config = SkillInstanceConfig::default();
        config.name = "Test".to_string();
        config.version = "1.0.0".to_string();
        config.entry_point = Some("/usr/bin/test".to_string());
        config.working_dir = Some("/tmp".to_string());
        config.env.insert("VAR".to_string(), "val".to_string());

        assert_eq!(config.name, "Test");
        assert_eq!(config.entry_point, Some("/usr/bin/test".to_string()));
        assert_eq!(config.working_dir, Some("/tmp".to_string()));
        assert_eq!(config.env.get("VAR"), Some(&"val".to_string()));
    }
}
