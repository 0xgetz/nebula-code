//! Agent orchestration engine with task scheduling, priority management, and dependency resolution.
//!
//! This module provides the core orchestration capabilities for managing multiple agents
//! and scheduling tasks with priority ordering and dependency tracking.

use crate::registry::{AgentRegistry, RegistryError};
use crate::types::{Agent, AgentId, AgentState};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur during orchestration operations.
#[derive(Debug, Error)]
pub enum OrchestrationError {
    #[error("Task not found: {0}")]
    TaskNotFound(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Invalid task state transition: {0}")]
    InvalidStateTransition(String),
    #[error("Dependency not satisfied: {0}")]
    DependencyNotSatisfied(String),
    #[error("Circular dependency detected")]
    CircularDependency,
    #[error("Registry error: {0}")]
    RegistryError(#[from] RegistryError),
    #[error("No available agent for task")]
    NoAvailableAgent,
    #[error("Task already has dependencies that conflict")]
    DependencyConflict,
}

/// Priority level for task scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    /// Low priority tasks, processed last.
    Low = 0,
    /// Normal priority tasks, default level.
    Normal = 1,
    /// High priority tasks, processed before normal.
    High = 2,
    /// Critical priority tasks, processed first.
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Status of a task in the orchestration system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is waiting to be scheduled or started.
    Pending,
    /// Task is currently being executed.
    Running,
    /// Task has completed successfully.
    Completed,
    /// Task execution failed.
    Failed,
    /// Task was cancelled by user or system.
    Cancelled,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl TaskStatus {
    /// Returns true if this is a terminal state (Completed, Failed, or Cancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    /// Returns true if the task can be transitioned to the given status.
    pub fn can_transition_to(&self, target: TaskStatus) -> bool {
        match (self, target) {
            (TaskStatus::Pending, TaskStatus::Running) => true,
            (TaskStatus::Pending, TaskStatus::Cancelled) => true,
            (TaskStatus::Running, TaskStatus::Completed) => true,
            (TaskStatus::Running, TaskStatus::Failed) => true,
            (TaskStatus::Running, TaskStatus::Cancelled) => true,
            _ => false,
        }
    }
}

/// Unique identifier for a task.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(Uuid);

impl TaskId {
    /// Creates a new random task ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a task ID from an existing UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a task to be executed by an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for this task.
    pub id: TaskId,
    /// Human-readable description of the task.
    pub description: String,
    /// The agent currently assigned to this task, if any.
    pub assigned_agent: Option<AgentId>,
    /// Priority level for scheduling.
    pub priority: Priority,
    /// IDs of tasks that must complete before this task can start.
    pub dependencies: Vec<TaskId>,
    /// Current status of the task.
    pub status: TaskStatus,
    /// Optional result or error message upon completion.
    pub result: Option<String>,
    /// Timestamp when the task was created (Unix timestamp).
    pub created_at: u64,
    /// Timestamp when the task was last updated (Unix timestamp).
    pub updated_at: u64,
}

impl Task {
    /// Creates a new task with the given description and default priority.
    pub fn new(description: impl Into<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            id: TaskId::new(),
            description: description.into(),
            assigned_agent: None,
            priority: Priority::default(),
            dependencies: Vec::new(),
            status: TaskStatus::default(),
            result: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the priority of the task.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self
    }

    /// Adds dependencies to the task.
    pub fn with_dependencies(mut self, dependencies: Vec<TaskId>) -> Self {
        self.dependencies = dependencies;
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self
    }

    /// Returns true if all dependencies are satisfied (completed).
    pub fn dependencies_satisfied(&self, task_status: &HashMap<TaskId, TaskStatus>) -> bool {
        self.dependencies.iter().all(|dep_id| {
            task_status.get(dep_id).map_or(false, |status| *status == TaskStatus::Completed)
        })
    }
}

/// Task scheduler responsible for managing task lifecycle and scheduling.
#[derive(Debug, Default)]
pub struct TaskScheduler {
    /// Map of task IDs to tasks.
    tasks: Arc<RwLock<HashMap<TaskId, Task>>>,
    /// Registry of available agents.
    registry: Arc<RwLock<AgentRegistry>>,
}

impl TaskScheduler {
    /// Creates a new task scheduler with an empty agent registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new task scheduler with a provided agent registry.
    pub fn with_registry(registry: AgentRegistry) -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(RwLock::new(registry)),
        }
    }

    /// Schedules a new task for execution.
    pub fn schedule_task(&self, task: Task) -> Result<TaskId, OrchestrationError> {
        // Validate dependencies exist
        {
            let tasks = self.tasks.read().map_err(|_| OrchestrationError::DependencyConflict)?;
            for dep_id in &task.dependencies {
                if !tasks.contains_key(dep_id) {
                    return Err(OrchestrationError::DependencyNotSatisfied(
                        format!("Dependency task {} not found", dep_id),
                    ));
                }
            }
        }

        // Check for circular dependencies
        self.check_circular_dependencies(&task.id, &task.dependencies)?;

        let task_id = task.id.clone();
        let mut tasks = self.tasks.write().map_err(|_| OrchestrationError::DependencyConflict)?;
        tasks.insert(task_id.clone(), task);
        Ok(task_id)
    }

    /// Cancels a pending or running task.
    pub fn cancel_task(&self, task_id: &TaskId) -> Result<(), OrchestrationError> {
        let mut tasks = self.tasks.write().map_err(|_| OrchestrationError::DependencyConflict)?;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| OrchestrationError::TaskNotFound(task_id.to_string()))?;

        if task.status.is_terminal() {
            return Err(OrchestrationError::InvalidStateTransition(format!(
                "Cannot cancel task in {:?} state",
                task.status
            )));
        }

        if !task.status.can_transition_to(TaskStatus::Cancelled) {
            return Err(OrchestrationError::InvalidStateTransition(format!(
                "Cannot transition from {:?} to Cancelled",
                task.status
            )));
        }

        task.status = TaskStatus::Cancelled;
        task.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(())
    }

    /// Gets the current status of a task.
    pub fn get_task_status(&self, task_id: &TaskId) -> Result<TaskStatus, OrchestrationError> {
        let tasks = self.tasks.read().map_err(|_| OrchestrationError::DependencyConflict)?;
        let task = tasks
            .get(task_id)
            .ok_or_else(|| OrchestrationError::TaskNotFound(task_id.to_string()))?;
        Ok(task.status)
    }

    /// Gets a full copy of a task.
    pub fn get_task(&self, task_id: &TaskId) -> Result<Task, OrchestrationError> {
        let tasks = self.tasks.read().map_err(|_| OrchestrationError::DependencyConflict)?;
        let task = tasks
            .get(task_id)
            .cloned()
            .ok_or_else(|| OrchestrationError::TaskNotFound(task_id.to_string()))?;
        Ok(task)
    }

    /// Assigns a task to a specific agent.
    pub fn assign_task(&self, task_id: &TaskId, agent_id: &AgentId) -> Result<(), OrchestrationError> {
        // Verify agent exists and is idle
        {
            let registry = self.registry.read().map_err(|_| OrchestrationError::RegistryError(RegistryError::RegistryLocked))?;
            let agent = registry.get_agent(agent_id)
                .map_err(OrchestrationError::RegistryError)?;
            if agent.state != AgentState::Idle {
                return Err(OrchestrationError::NoAvailableAgent);
            }
        }

        // First, check status and dependencies without holding a mutable lock
        {
            let tasks = self.tasks.read().map_err(|_| OrchestrationError::DependencyConflict)?;
            let task = tasks
                .get(task_id)
                .ok_or_else(|| OrchestrationError::TaskNotFound(task_id.to_string()))?;

            if task.status != TaskStatus::Pending {
                return Err(OrchestrationError::InvalidStateTransition(format!(
                    "Can only assign pending tasks, current status: {:?}",
                    task.status
                )));
            }

            // Check dependencies are satisfied
            let task_status_map: HashMap<TaskId, TaskStatus> = tasks.iter().map(|(id, t)| (id.clone(), t.status)).collect();
            if !task.dependencies_satisfied(&task_status_map) {
                return Err(OrchestrationError::DependencyNotSatisfied(
                    "Not all dependencies are completed".to_string(),
                ));
            }
        }

        // Now acquire mutable lock and update
        let mut tasks = self.tasks.write().map_err(|_| OrchestrationError::DependencyConflict)?;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| OrchestrationError::TaskNotFound(task_id.to_string()))?;

        task.assigned_agent = Some(agent_id.clone());
        task.status = TaskStatus::Running;
        task.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(())
    }

    /// Marks a task as completed with an optional result.
    pub fn complete_task(&self, task_id: &TaskId, result: Option<String>) -> Result<(), OrchestrationError> {
        let mut tasks = self.tasks.write().map_err(|_| OrchestrationError::DependencyConflict)?;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| OrchestrationError::TaskNotFound(task_id.to_string()))?;

        if task.status != TaskStatus::Running {
            return Err(OrchestrationError::InvalidStateTransition(format!(
                "Can only complete running tasks, current status: {:?}",
                task.status
            )));
        }

        task.status = TaskStatus::Completed;
        task.result = result;
        task.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(())
    }

    /// Marks a task as failed with an error message.
    pub fn fail_task(&self, task_id: &TaskId, error: String) -> Result<(), OrchestrationError> {
        let mut tasks = self.tasks.write().map_err(|_| OrchestrationError::DependencyConflict)?;
        let task = tasks
            .get_mut(task_id)
            .ok_or_else(|| OrchestrationError::TaskNotFound(task_id.to_string()))?;

        if task.status != TaskStatus::Running {
            return Err(OrchestrationError::InvalidStateTransition(format!(
                "Can only fail running tasks, current status: {:?}",
                task.status
            )));
        }

        task.status = TaskStatus::Failed;
        task.result = Some(error);
        task.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(())
    }

    /// Returns all pending tasks sorted by priority (highest first) and creation time.
    pub fn get_pending_tasks(&self) -> Result<Vec<Task>, OrchestrationError> {
        let tasks = self.tasks.read().map_err(|_| OrchestrationError::DependencyConflict)?;
        let mut pending: Vec<Task> = tasks
            .values()
            .filter(|t| t.status == TaskStatus::Pending)
            .cloned()
            .collect();

        // Sort by priority (descending) then by created_at (ascending - oldest first)
        pending.sort_by(|a, b| {
            b.priority.cmp(&a.priority).then_with(|| a.created_at.cmp(&b.created_at))
        });

        Ok(pending)
    }

    /// Returns all tasks with a specific status.
    pub fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>, OrchestrationError> {
        let tasks = self.tasks.read().map_err(|_| OrchestrationError::DependencyConflict)?;
        Ok(tasks.values().filter(|t| t.status == status).cloned().collect())
    }

    /// Returns all tasks assigned to a specific agent.
    pub fn get_tasks_by_agent(&self, agent_id: &AgentId) -> Result<Vec<Task>, OrchestrationError> {
        let tasks = self.tasks.read().map_err(|_| OrchestrationError::DependencyConflict)?;
        Ok(tasks
            .values()
            .filter(|t| t.assigned_agent.as_ref() == Some(agent_id))
            .cloned()
            .collect())
    }

    /// Gets the agent registry.
    pub fn registry(&self) -> &Arc<RwLock<AgentRegistry>> {
        &self.registry
    }

    /// Returns the total number of tasks.
    pub fn task_count(&self) -> Result<usize, OrchestrationError> {
        let tasks = self.tasks.read().map_err(|_| OrchestrationError::DependencyConflict)?;
        Ok(tasks.len())
    }

    /// Checks for circular dependencies using DFS.
    fn check_circular_dependencies(&self, new_task_id: &TaskId, dependencies: &[TaskId]) -> Result<(), OrchestrationError> {
        let tasks = self.tasks.read().map_err(|_| OrchestrationError::DependencyConflict)?;
        let mut visited = std::collections::HashSet::new();
        let mut path = std::collections::HashSet::new();

        for dep_id in dependencies {
            if self.has_cycle(dep_id, &tasks, &mut visited, &mut path, new_task_id)? {
                return Err(OrchestrationError::CircularDependency);
            }
        }
        Ok(())
    }

    fn has_cycle(
        &self,
        task_id: &TaskId,
        tasks: &HashMap<TaskId, Task>,
        visited: &mut std::collections::HashSet<TaskId>,
        path: &mut std::collections::HashSet<TaskId>,
        new_task_id: &TaskId,
    ) -> Result<bool, OrchestrationError> {
        if path.contains(task_id) {
            return Ok(true);
        }
        if visited.contains(task_id) {
            return Ok(false);
        }

        visited.insert(task_id.clone());
        path.insert(task_id.clone());

        if let Some(task) = tasks.get(task_id) {
            for dep_id in &task.dependencies {
                if dep_id == new_task_id {
                    return Ok(true); // Would create a cycle
                }
                if self.has_cycle(dep_id, tasks, visited, path, new_task_id)? {
                    return Ok(true);
                }
            }
        }

        path.remove(task_id);
        Ok(false)
    }
}

/// Main orchestrator that manages multiple agents and coordinates task execution.
#[derive(Debug)]
pub struct Orchestrator {
    /// The task scheduler.
    scheduler: TaskScheduler,
    /// Name of the orchestrator instance.
    name: String,
}

impl Orchestrator {
    /// Creates a new orchestrator with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            scheduler: TaskScheduler::new(),
            name: name.into(),
        }
    }

    /// Creates an orchestrator with a pre-configured agent registry.
    pub fn with_registry(name: impl Into<String>, registry: AgentRegistry) -> Self {
        Self {
            scheduler: TaskScheduler::with_registry(registry),
            name: name.into(),
        }
    }

    /// Returns the name of the orchestrator.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the task scheduler.
    pub fn scheduler(&self) -> &TaskScheduler {
        &self.scheduler
    }

    /// Registers an agent with the orchestrator.
    pub fn register_agent(&self, agent: Agent) -> Result<(), OrchestrationError> {
        let registry = self.scheduler.registry();
        let reg = registry.write().map_err(|_| OrchestrationError::RegistryError(RegistryError::RegistryLocked))?;
        reg.register(agent).map_err(OrchestrationError::RegistryError)?;
        Ok(())
    }

    /// Unregisters an agent from the orchestrator.
    pub fn unregister_agent(&self, agent_id: &AgentId) -> Result<Agent, OrchestrationError> {
        let registry = self.scheduler.registry();
        let reg = registry.write().map_err(|_| OrchestrationError::RegistryError(RegistryError::RegistryLocked))?;
        reg.unregister(agent_id).map_err(OrchestrationError::RegistryError)
    }

    /// Schedules a new task.
    pub fn schedule_task(&self, task: Task) -> Result<TaskId, OrchestrationError> {
        self.scheduler.schedule_task(task)
    }

    /// Finds an available idle agent that has the required capability.
    pub fn find_available_agent(&self, capability: &str) -> Result<AgentId, OrchestrationError> {
        let registry = self.scheduler.registry();
        let reg = registry.read().map_err(|_| OrchestrationError::RegistryError(RegistryError::RegistryLocked))?;
        
        let agents = reg.find_by_capability(capability).map_err(OrchestrationError::RegistryError)?;
        let idle_agent = agents.iter().find(|a| a.state == AgentState::Idle)
            .ok_or(OrchestrationError::NoAvailableAgent)?;
        
        Ok(idle_agent.id.clone())
    }

    /// Gets all registered agents.
    pub fn list_agents(&self) -> Result<Vec<Agent>, OrchestrationError> {
        let registry = self.scheduler.registry();
        let reg = registry.read().map_err(|_| OrchestrationError::RegistryError(RegistryError::RegistryLocked))?;
        reg.list_agents().map_err(OrchestrationError::RegistryError)
    }

    /// Gets the next pending task that is ready to execute (dependencies satisfied),
    /// respecting priority ordering.
    pub fn get_next_ready_task(&self) -> Result<Option<Task>, OrchestrationError> {
        let pending = self.scheduler.get_pending_tasks()?;
        
        // Build a map of all task statuses for dependency checking
        let all_tasks = self.scheduler.tasks.read().map_err(|_| OrchestrationError::DependencyConflict)?;
        let status_map: HashMap<TaskId, TaskStatus> = all_tasks.iter().map(|(id, t)| (id.clone(), t.status)).collect();
        
        for task in pending {
            if task.dependencies_satisfied(&status_map) {
                return Ok(Some(task));
            }
        }
        
        Ok(None)
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new("default")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AgentCapability;
    use crate::types::AgentMetadata;

    fn create_test_agent(name: &str, capabilities: &[&str]) -> Agent {
        let id = AgentId::new();
        let mut agent = Agent::new(id, AgentMetadata::new().with_name(name));
        agent.set_state(AgentState::Idle);
        for cap in capabilities {
            agent.add_capability(AgentCapability::new(*cap, "Test capability", "1.0"));
        }
        agent
    }

    // ===== Priority Ordering Tests =====

    #[test]
    fn test_priority_ordering() {
        let scheduler = TaskScheduler::new();
        
        let low = Task::new("Low priority task").with_priority(Priority::Low);
        let high = Task::new("High priority task").with_priority(Priority::High);
        let normal = Task::new("Normal priority task").with_priority(Priority::Normal);
        let critical = Task::new("Critical priority task").with_priority(Priority::Critical);

        let low_id = scheduler.schedule_task(low).unwrap();
        let high_id = scheduler.schedule_task(high).unwrap();
        let normal_id = scheduler.schedule_task(normal).unwrap();
        let critical_id = scheduler.schedule_task(critical).unwrap();

        let pending = scheduler.get_pending_tasks().unwrap();
        
        assert_eq!(pending[0].id, critical_id);
        assert_eq!(pending[1].id, high_id);
        assert_eq!(pending[2].id, normal_id);
        assert_eq!(pending[3].id, low_id);
    }

    #[test]
    fn test_priority_enum_ordering() {
        assert!(Priority::Critical > Priority::High);
        assert!(Priority::High > Priority::Normal);
        assert!(Priority::Normal > Priority::Low);
    }

    // ===== Task Scheduling Tests =====

    #[test]
    fn test_schedule_task_basic() {
        let scheduler = TaskScheduler::new();
        let task = Task::new("Test task");
        
        let task_id = scheduler.schedule_task(task).unwrap();
        assert_eq!(scheduler.task_count().unwrap(), 1);
        
        let status = scheduler.get_task_status(&task_id).unwrap();
        assert_eq!(status, TaskStatus::Pending);
    }

    #[test]
    fn test_cancel_task() {
        let scheduler = TaskScheduler::new();
        let task = Task::new("To be cancelled");
        let task_id = scheduler.schedule_task(task).unwrap();

        scheduler.cancel_task(&task_id).unwrap();
        assert_eq!(scheduler.get_task_status(&task_id).unwrap(), TaskStatus::Cancelled);

        assert!(scheduler.cancel_task(&task_id).is_err());
    }

    #[test]
    fn test_get_task_status_nonexistent() {
        let scheduler = TaskScheduler::new();
        let fake_id = TaskId::new();
        assert!(scheduler.get_task_status(&fake_id).is_err());
    }

    // ===== Agent Assignment Tests =====

    #[test]
    fn test_assign_task_to_agent() {
        let mut registry = AgentRegistry::new();
        let agent = create_test_agent("Worker", &["coding"]);
        let agent_id = agent.id.clone();
        registry.register(agent).unwrap();

        let scheduler = TaskScheduler::with_registry(registry);
        let task = Task::new("Write code");
        let task_id = scheduler.schedule_task(task).unwrap();

        scheduler.assign_task(&task_id, &agent_id).unwrap();
        
        let task = scheduler.get_task(&task_id).unwrap();
        assert_eq!(task.assigned_agent, Some(agent_id));
        assert_eq!(task.status, TaskStatus::Running);
    }

    #[test]
    fn test_assign_task_nonexistent_agent() {
        let scheduler = TaskScheduler::new();
        let task = Task::new("Test task");
        let task_id = scheduler.schedule_task(task).unwrap();
        let fake_agent_id = AgentId::new();

        assert!(scheduler.assign_task(&task_id, &fake_agent_id).is_err());
    }

    #[test]
    fn test_assign_task_already_running() {
        let mut registry = AgentRegistry::new();
        let agent = create_test_agent("Worker", &["coding"]);
        let agent_id = agent.id.clone();
        registry.register(agent).unwrap();

        let scheduler = TaskScheduler::with_registry(registry);
        let task = Task::new("Test task");
        let task_id = scheduler.schedule_task(task).unwrap();

        scheduler.assign_task(&task_id, &agent_id).unwrap();
        assert!(scheduler.assign_task(&task_id, &agent_id).is_err());
    }

    // ===== Complete and Fail Task Tests =====

    #[test]
    fn test_complete_task() {
        let mut registry = AgentRegistry::new();
        let agent = create_test_agent("Worker", &["testing"]);
        let agent_id = agent.id.clone();
        registry.register(agent).unwrap();

        let scheduler = TaskScheduler::with_registry(registry);
        let task = Task::new("Run tests");
        let task_id = scheduler.schedule_task(task).unwrap();
        scheduler.assign_task(&task_id, &agent_id).unwrap();

        scheduler.complete_task(&task_id, Some("All tests passed".to_string())).unwrap();
        
        assert_eq!(scheduler.get_task_status(&task_id).unwrap(), TaskStatus::Completed);
        let task = scheduler.get_task(&task_id).unwrap();
        assert_eq!(task.result, Some("All tests passed".to_string()));
    }

    #[test]
    fn test_fail_task() {
        let mut registry = AgentRegistry::new();
        let agent = create_test_agent("Worker", &["testing"]);
        let agent_id = agent.id.clone();
        registry.register(agent).unwrap();

        let scheduler = TaskScheduler::with_registry(registry);
        let task = Task::new("Run tests");
        let task_id = scheduler.schedule_task(task).unwrap();
        scheduler.assign_task(&task_id, &agent_id).unwrap();

        scheduler.fail_task(&task_id, "Test suite failed: 3 failures".to_string()).unwrap();
        
        assert_eq!(scheduler.get_task_status(&task_id).unwrap(), TaskStatus::Failed);
        let task = scheduler.get_task(&task_id).unwrap();
        assert_eq!(task.result, Some("Test suite failed: 3 failures".to_string()));
    }

    #[test]
    fn test_complete_non_running_task_fails() {
        let scheduler = TaskScheduler::new();
        let task = Task::new("Test task");
        let task_id = scheduler.schedule_task(task).unwrap();

        assert!(scheduler.complete_task(&task_id, None).is_err());
    }

    // ===== Dependency Management Tests =====

    #[test]
    fn test_task_with_dependencies() {
        let scheduler = TaskScheduler::new();
        
        let task1 = Task::new("Setup environment");
        let task1_id = scheduler.schedule_task(task1).unwrap();

        let task2 = Task::new("Build project").with_dependencies(vec![task1_id.clone()]);
        let task2_id = scheduler.schedule_task(task2).unwrap();

        let task2 = scheduler.get_task(&task2_id).unwrap();
        assert_eq!(task2.dependencies.len(), 1);
        assert_eq!(task2.dependencies[0], task1_id);
    }

    #[test]
    fn test_dependencies_satisfied_check() {
        let scheduler = TaskScheduler::new();
        
        let task1 = Task::new("Task 1");
        let task1_id = scheduler.schedule_task(task1).unwrap();

        let task2 = Task::new("Task 2").with_dependencies(vec![task1_id.clone()]);
        let task2_id = scheduler.schedule_task(task2).unwrap();

        let mut status_map = HashMap::new();
        status_map.insert(task1_id.clone(), TaskStatus::Pending);
        status_map.insert(task2_id.clone(), TaskStatus::Pending);

        let task2 = scheduler.get_task(&task2_id).unwrap();
        assert!(!task2.dependencies_satisfied(&status_map));

        status_map.insert(task1_id, TaskStatus::Completed);
        assert!(task2.dependencies_satisfied(&status_map));
    }

    #[test]
    fn test_cannot_assign_task_with_unsatisfied_dependencies() {
        let mut registry = AgentRegistry::new();
        let agent = create_test_agent("Worker", &["build"]);
        let agent_id = agent.id.clone();
        registry.register(agent).unwrap();

        let scheduler = TaskScheduler::with_registry(registry);
        
        let task1 = Task::new("Compile");
        let task1_id = scheduler.schedule_task(task1).unwrap();

        let task2 = Task::new("Link").with_dependencies(vec![task1_id]);
        let task2_id = scheduler.schedule_task(task2).unwrap();

        assert!(scheduler.assign_task(&task2_id, &agent_id).is_err());
    }

    #[test]
    fn test_dependency_not_found_error() {
        let scheduler = TaskScheduler::new();
        let fake_dep = TaskId::new();
        let task = Task::new("Test").with_dependencies(vec![fake_dep]);
        
        assert!(scheduler.schedule_task(task).is_err());
    }

    #[test]
    fn test_get_pending_tasks_empty() {
        let scheduler = TaskScheduler::new();
        let pending = scheduler.get_pending_tasks().unwrap();
        assert!(pending.is_empty());
    }

    #[test]
    fn test_get_pending_tasks_excludes_completed() {
        let mut registry = AgentRegistry::new();
        let agent = create_test_agent("Worker", &["work"]);
        let agent_id = agent.id.clone();
        registry.register(agent).unwrap();

        let scheduler = TaskScheduler::with_registry(registry);
        
        let task1 = Task::new("Done task");
        let task1_id = scheduler.schedule_task(task1).unwrap();
        scheduler.assign_task(&task1_id, &agent_id).unwrap();
        scheduler.complete_task(&task1_id, None).unwrap();

        let task2 = Task::new("Pending task");
        scheduler.schedule_task(task2).unwrap();

        let pending = scheduler.get_pending_tasks().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].description, "Pending task");
    }

    // ===== Orchestrator Integration Tests =====

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = Orchestrator::new("test-orchestrator");
        assert_eq!(orchestrator.name(), "test-orchestrator");
    }

    #[test]
    fn test_orchestrator_register_and_list_agents() {
        let orchestrator = Orchestrator::new("test");
        
        let agent1 = create_test_agent("Agent1", &["coding"]);
        let agent2 = create_test_agent("Agent2", &["testing"]);
        
        orchestrator.register_agent(agent1).unwrap();
        orchestrator.register_agent(agent2).unwrap();

        let agents = orchestrator.list_agents().unwrap();
        assert_eq!(agents.len(), 2);
    }

    #[test]
    fn test_orchestrator_schedule_and_get_next_ready_task() {
        let orchestrator = Orchestrator::new("test");
        
        let agent = create_test_agent("Builder", &["build"]);
        let agent_id = agent.id.clone();
        orchestrator.register_agent(agent).unwrap();

        let task1 = Task::new("Prepare");
        let task1_id = orchestrator.schedule_task(task1).unwrap();

        let task2 = Task::new("Build").with_dependencies(vec![task1_id.clone()]);
        orchestrator.schedule_task(task2).unwrap();

        let next = orchestrator.get_next_ready_task().unwrap();
        assert!(next.is_some());
        assert_eq!(next.unwrap().description, "Prepare");

        orchestrator.scheduler.assign_task(&task1_id, &agent_id).unwrap();
        orchestrator.scheduler.complete_task(&task1_id, Some("done".to_string())).unwrap();

        let next = orchestrator.get_next_ready_task().unwrap();
        assert!(next.is_some());
        assert_eq!(next.unwrap().description, "Build");
    }

    #[test]
    fn test_orchestrator_find_available_agent() {
        let orchestrator = Orchestrator::new("test");
        
        let agent = create_test_agent("Coder", &["rust", "python"]);
        let agent_id = agent.id.clone();
        orchestrator.register_agent(agent).unwrap();

        let found = orchestrator.find_available_agent("rust").unwrap();
        assert_eq!(found, agent_id);

        assert!(orchestrator.find_available_agent("nonexistent").is_err());
    }

    #[test]
    fn test_task_status_transitions() {
        assert!(TaskStatus::Pending.can_transition_to(TaskStatus::Running));
        assert!(TaskStatus::Pending.can_transition_to(TaskStatus::Cancelled));
        assert!(!TaskStatus::Pending.can_transition_to(TaskStatus::Completed));
        
        assert!(TaskStatus::Running.can_transition_to(TaskStatus::Completed));
        assert!(TaskStatus::Running.can_transition_to(TaskStatus::Failed));
        assert!(TaskStatus::Running.can_transition_to(TaskStatus::Cancelled));
        assert!(!TaskStatus::Running.can_transition_to(TaskStatus::Pending));

        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(!TaskStatus::Pending.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
    }

    #[test]
    fn test_get_tasks_by_status() {
        let scheduler = TaskScheduler::new();
        
        scheduler.schedule_task(Task::new("Task 1")).unwrap();
        scheduler.schedule_task(Task::new("Task 2")).unwrap();

        let pending = scheduler.get_tasks_by_status(TaskStatus::Pending).unwrap();
        assert_eq!(pending.len(), 2);

        let running = scheduler.get_tasks_by_status(TaskStatus::Running).unwrap();
        assert!(running.is_empty());
    }

    #[test]
    fn test_get_tasks_by_agent() {
        let mut registry = AgentRegistry::new();
        let agent = create_test_agent("Worker", &["work"]);
        let agent_id = agent.id.clone();
        registry.register(agent).unwrap();

        let scheduler = TaskScheduler::with_registry(registry);
        
        let task1 = Task::new("Task 1");
        let task1_id = scheduler.schedule_task(task1).unwrap();
        let task2 = Task::new("Task 2");
        let task2_id = scheduler.schedule_task(task2).unwrap();

        scheduler.assign_task(&task1_id, &agent_id).unwrap();

        let agent_tasks = scheduler.get_tasks_by_agent(&agent_id).unwrap();
        assert_eq!(agent_tasks.len(), 1);
        assert_eq!(agent_tasks[0].id, task1_id);

        let no_tasks = scheduler.get_tasks_by_agent(&AgentId::new()).unwrap();
        assert!(no_tasks.is_empty());
    }
}
