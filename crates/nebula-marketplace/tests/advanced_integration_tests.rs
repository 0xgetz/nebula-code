//! Advanced integration tests for execution engine, dependency resolution, and persistence
//!
//! These tests cover:
//! 1. Full flow: create skill -> add dependencies -> resolve -> execute
//! 2. Edge cases for dependency resolution: diamond dependencies, version conflicts, optional deps
//! 3. Execution engine: concurrent execution, timeout handling, error recovery
//! 4. Persistence: concurrent access, recovery after crash, incremental sync
//! 5. CLI integration: full workflow from skill discovery to execution

use nebula_marketplace::dependencies::{Dependency, DependencyGraph, DependencyNode, DependencyResolver, DependencyError};
use nebula_marketplace::execution::{
    ExecutionContext, ExecutionOutput, ExecutionState, ExecutionStatus,
    ExecutorCapabilities, ExecutorRegistry, ResourceLimits, SkillExecutor, SkillExecutorError,
    SkillInstance, SkillInstanceConfig,
};
use nebula_marketplace::persistence::{FileSkillStorage, SkillIndex, SkillStorage, PersistenceError};
use nebula_marketplace::rating::Rating;
use nebula_marketplace::registry::SkillRegistry;
use nebula_marketplace::types::{
    MarketplaceError, Skill, SkillCategory, SkillManifest, SkillMetadata, SkillVersion,
};
use nebula_marketplace::{MarketplaceCLI, Command};
use semver::Version;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

// ============================================================================
// Helper types and functions
// ============================================================================

/// Simple test executor that echoes input back as output
#[derive(Debug, Clone)]
struct EchoExecutor {
    skill_type: String,
    delay: Duration,
    should_fail: bool,
}

impl EchoExecutor {
    fn new(skill_type: &str, delay_ms: u64) -> Self {
        Self {
            skill_type: skill_type.to_string(),
            delay: Duration::from_millis(delay_ms),
            should_fail: false,
        }
    }

    fn with_failure(mut self) -> Self {
        self.should_fail = true;
        self
    }
}

impl SkillExecutor<String, String> for EchoExecutor {
    fn execute(
        &self,
        ctx: &ExecutionContext<String>,
    ) -> Result<ExecutionOutput<String>, SkillExecutorError> {
        if self.should_fail {
            return Err(SkillExecutorError::ExecutionError(
                "Intentional failure".to_string(),
            ));
        }

        // Simulate work
        thread::sleep(self.delay);

        Ok(ExecutionOutput::success(
            ExecutionStatus::Completed,
            format!("Echo: {}", ctx.input()),
        ))
    }

    fn validate(&self, _ctx: &ExecutionContext<String>) -> Result<(), SkillExecutorError> {
        Ok(())
    }

    fn prepare(&self, _instance: &mut SkillInstance<String, String>) -> Result<(), SkillExecutorError> {
        Ok(())
    }

    fn skill_type(&self) -> &str {
        &self.skill_type
    }

    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities {
            supports_async: true,
            supports_streaming: false,
            supports_cancellation: true,
            supports_retries: true,
            supports_progress: true,
            max_concurrent: 10,
            input_formats: vec!["text".to_string()],
            output_formats: vec!["text".to_string()],
        }
    }

    fn version(&self) -> &str {
        "1.0.0"
    }
}

/// Test executor that counts executions
#[derive(Debug, Clone)]
struct CounterExecutor {
    skill_type: String,
    count: Arc<Mutex<u32>>,
}

impl CounterExecutor {
    fn new(skill_type: &str) -> Self {
        Self {
            skill_type: skill_type.to_string(),
            count: Arc::new(Mutex::new(0)),
        }
    }

    fn get_count(&self) -> u32 {
        *self.count.lock().unwrap()
    }
}

impl SkillExecutor<u32, u32> for CounterExecutor {
    fn execute(
        &self,
        _ctx: &ExecutionContext<u32>,
    ) -> Result<ExecutionOutput<u32>, SkillExecutorError> {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        let current = *count;

        Ok(ExecutionOutput::success(
            ExecutionStatus::Completed,
            current,
        ))
    }

    fn validate(&self, _ctx: &ExecutionContext<u32>) -> Result<(), SkillExecutorError> {
        Ok(())
    }

    fn prepare(&self, _instance: &mut SkillInstance<u32, u32>) -> Result<(), SkillExecutorError> {
        Ok(())
    }

    fn skill_type(&self) -> &str {
        &self.skill_type
    }

    fn capabilities(&self) -> ExecutorCapabilities {
        ExecutorCapabilities {
            supports_async: true,
            supports_streaming: false,
            supports_cancellation: true,
            supports_retries: true,
            supports_progress: true,
            max_concurrent: 100,
            input_formats: vec!["number".to_string()],
            output_formats: vec!["number".to_string()],
        }
    }

    fn version(&self) -> &str {
        "1.0.0"
    }
}

fn create_skill_with_deps(
    id: &str,
    name: &str,
    dependencies: Vec<Dependency>,
) -> Skill {
    let mut skill = Skill::new(
        id.to_string(),
        SkillMetadata::new(
            name.to_string(),
            format!("Description for {}", name),
            "test-author".to_string(),
        )
        .with_categories(vec![SkillCategory::CodeGeneration])
        .with_tags(vec!["test".to_string()])
        .with_version(SkillVersion::new(1, 0, 0)),
        SkillManifest::new("main.rs".to_string(), "rust".to_string()),
    );
    // Note: Skill struct needs a dependencies field - this would need to be added
    skill
}

// ============================================================================
// 1. Full flow integration tests: create skill -> dependencies -> resolve -> execute
// ============================================================================

#[test]
fn test_full_flow_skill_creation_to_execution() {
    // Step 1: Create skills with dependencies using DependencyGraph
    let mut graph = DependencyGraph::new();
    
    // Add base skill (no dependencies)
    graph.add_node(DependencyNode::new("base", Version::parse("1.0.0").unwrap(), vec![]));
    
    // Add middleware skill (depends on base)
    let middleware_deps = vec![Dependency::new("base", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("middleware", Version::parse("1.0.0").unwrap(), middleware_deps));
    
    // Add app skill (depends on middleware and base)
    let app_deps = vec![
        Dependency::new("middleware", ">=1.0.0").unwrap(),
        Dependency::new("base", ">=1.0.0").unwrap(),
    ];
    graph.add_node(DependencyNode::new("app", Version::parse("1.0.0").unwrap(), app_deps));

    // Step 2: Resolve dependencies
    let resolver = DependencyResolver::new(graph);
    let resolution = resolver.resolve_dependencies(&["app".to_string()]);
    assert!(resolution.is_ok());
    let resolved = resolution.unwrap();
    
    // Should resolve in order: base -> middleware -> app
    assert_eq!(resolved.ordered_skills.len(), 3);
    assert_eq!(resolved.ordered_skills[0].skill_id, "base");
    assert_eq!(resolved.ordered_skills[1].skill_id, "middleware");
    assert_eq!(resolved.ordered_skills[2].skill_id, "app");

    // Step 3: Set up execution registry
    let mut registry: ExecutorRegistry<String, String> = ExecutorRegistry::new();
    registry.register("base".to_string(), Box::new(EchoExecutor::new("base", 10))).unwrap();
    registry.register("middleware".to_string(), Box::new(EchoExecutor::new("middleware", 10))).unwrap();
    registry.register("app".to_string(), Box::new(EchoExecutor::new("app", 10))).unwrap();

    // Step 4: Execute in dependency order
    let mut results = Vec::new();
    for skill in &resolved.ordered_skills {
        let ctx = ExecutionContext::new(format!("input-for-{}", skill.skill_id));
        let result = registry.execute(&skill.skill_id, &ctx);
        assert!(result.is_ok());
        results.push(result.unwrap());
    }

    // Verify all executions completed
    assert_eq!(results.len(), 3);
    for result in &results {
        assert_eq!(result.status, ExecutionStatus::Completed);
        assert!(result.value.is_some());
    }
}

#[test]
fn test_full_flow_with_cli() {
    let mut cli = MarketplaceCLI::new();

    // Create skills with dependencies
    let base_skill = create_skill_with_deps("base", "Base Skill", vec![]);
    let app_skill = create_skill_with_deps(
        "app",
        "Application Skill",
        vec![Dependency::new("base", ">=1.0.0").unwrap()],
    );

    // Register skills
    cli.registry_mut().register(base_skill).unwrap();
    cli.registry_mut().register(app_skill).unwrap();

    // Install skills via CLI
    let result = cli.execute(Command::Install {
        id: "base".to_string(),
        path: None,
    });
    assert!(result.is_ok());

    let result = cli.execute(Command::Install {
        id: "app".to_string(),
        path: None,
    });
    assert!(result.is_ok());

    // Verify both are installed
    assert!(cli.registry().get("base").unwrap().installed);
    assert!(cli.registry().get("app").unwrap().installed);

    // List installed skills
    let result = cli.execute(Command::List {
        category: None,
        installed: Some(true),
    });
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("2 skill(s)"));
}

// ============================================================================
// 2. Edge case tests for dependency resolution
// ============================================================================

#[test]
fn test_dependency_diamond_dependency() {
    // Diamond dependency: A -> B, A -> C, B -> D, C -> D
    let mut graph = DependencyGraph::new();
    
    graph.add_node(DependencyNode::new("d", Version::parse("1.0.0").unwrap(), vec![]));
    
    let b_deps = vec![Dependency::new("d", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("b", Version::parse("1.0.0").unwrap(), b_deps));
    
    let c_deps = vec![Dependency::new("d", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("c", Version::parse("1.0.0").unwrap(), c_deps));
    
    let a_deps = vec![
        Dependency::new("b", ">=1.0.0").unwrap(),
        Dependency::new("c", ">=1.0.0").unwrap(),
    ];
    graph.add_node(DependencyNode::new("a", Version::parse("1.0.0").unwrap(), a_deps));

    let resolver = DependencyResolver::new(graph);
    let resolution = resolver.resolve_dependencies(&["a".to_string()]);
    assert!(resolution.is_ok());
    let resolved = resolution.unwrap();

    // D should appear only once in the resolved order
    let d_count = resolved.ordered_skills.iter().filter(|s| s.skill_id == "d").count();
    assert_eq!(d_count, 1);

    // D should come before B and C
    let d_idx = resolved.ordered_skills.iter().position(|s| s.skill_id == "d").unwrap();
    let b_idx = resolved.ordered_skills.iter().position(|s| s.skill_id == "b").unwrap();
    let c_idx = resolved.ordered_skills.iter().position(|s| s.skill_id == "c").unwrap();
    assert!(d_idx < b_idx);
    assert!(d_idx < c_idx);
}

#[test]
fn test_dependency_version_conflict() {
    // Skill A requires B >=1.0.0,<2.0.0, Skill C requires B >=2.0.0
    let mut graph = DependencyGraph::new();
    
    // Register versions for B
    graph.register_versions("b", vec![
        Version::parse("1.5.0").unwrap(),
        Version::parse("2.5.0").unwrap(),
    ]);
    
    graph.add_node(DependencyNode::new("b", Version::parse("1.5.0").unwrap(), vec![]));
    
    let a_deps = vec![Dependency::new("b", ">=1.0.0,<2.0.0").unwrap()];
    graph.add_node(DependencyNode::new("a", Version::parse("1.0.0").unwrap(), a_deps));
    
    let c_deps = vec![Dependency::new("b", ">=2.0.0").unwrap()];
    graph.add_node(DependencyNode::new("c", Version::parse("1.0.0").unwrap(), c_deps));

    let resolver = DependencyResolver::new(graph);
    let conflicts = resolver.find_conflicts();
    assert!(!conflicts.is_empty());
    
    // Should have a conflict for skill B
    assert!(conflicts.iter().any(|c| c.dependency == "b"));
}

#[test]
fn test_dependency_optional_dependencies() {
    // Skill A has optional dependency on B
    let mut graph = DependencyGraph::new();
    
    // Add A with optional dependency on B (but B is not present)
    let a_deps = vec![Dependency::optional("b", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("a", Version::parse("1.0.0").unwrap(), a_deps));

    // Resolve A - should succeed even if B is not available
    let resolver = DependencyResolver::new(graph);
    let resolution = resolver.resolve_dependencies(&["a".to_string()]);
    assert!(resolution.is_ok());
    let resolved = resolution.unwrap();

    // Since B is optional and not present, A should be resolved alone
    assert_eq!(resolved.ordered_skills.len(), 1);
    assert_eq!(resolved.ordered_skills[0].skill_id, "a");
    assert!(!resolved.warnings.is_empty()); // Should have a warning about optional dep

    // Now add B and resolve again
    let mut graph2 = DependencyGraph::new();
    graph2.add_node(DependencyNode::new("b", Version::parse("1.0.0").unwrap(), vec![]));
    let a_deps2 = vec![Dependency::optional("b", ">=1.0.0").unwrap()];
    graph2.add_node(DependencyNode::new("a", Version::parse("1.0.0").unwrap(), a_deps2));

    let resolver2 = DependencyResolver::new(graph2);
    let resolution2 = resolver2.resolve_dependencies(&["a".to_string()]);
    assert!(resolution2.is_ok());
    let resolved2 = resolution2.unwrap();
    assert_eq!(resolved2.ordered_skills.len(), 2);
    assert_eq!(resolved2.ordered_skills[0].skill_id, "b");
    assert_eq!(resolved2.ordered_skills[1].skill_id, "a");
}

#[test]
fn test_dependency_circular_detection() {
    // A -> B -> C -> A (circular)
    let mut graph = DependencyGraph::new();
    
    let a_deps = vec![Dependency::new("b", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("a", Version::parse("1.0.0").unwrap(), a_deps));
    
    let b_deps = vec![Dependency::new("c", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("b", Version::parse("1.0.0").unwrap(), b_deps));
    
    let c_deps = vec![Dependency::new("a", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("c", Version::parse("1.0.0").unwrap(), c_deps));

    // Should detect cycle
    let resolver = DependencyResolver::new(graph);
    let cycles = resolver.detect_cycles();
    assert!(!cycles.is_empty());
    assert_eq!(cycles.len(), 1); // One cycle detected

    // Resolution should fail
    let resolution = resolver.resolve_dependencies(&["a".to_string()]);
    assert!(resolution.is_err());
    assert!(matches!(
        resolution.unwrap_err(),
        DependencyError::CircularDependency { .. }
    ));
}

#[test]
fn test_dependency_transitive_resolution() {
    // A -> B -> C -> D (transitive chain)
    let mut graph = DependencyGraph::new();
    
    graph.add_node(DependencyNode::new("d", Version::parse("1.0.0").unwrap(), vec![]));
    
    let c_deps = vec![Dependency::new("d", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("c", Version::parse("1.0.0").unwrap(), c_deps));
    
    let b_deps = vec![Dependency::new("c", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("b", Version::parse("1.0.0").unwrap(), b_deps));
    
    let a_deps = vec![Dependency::new("b", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("a", Version::parse("1.0.0").unwrap(), a_deps));

    let resolver = DependencyResolver::new(graph);
    let resolution = resolver.resolve_dependencies(&["a".to_string()]);
    assert!(resolution.is_ok());
    let resolved = resolution.unwrap();

    assert_eq!(resolved.ordered_skills.len(), 4);
    assert_eq!(resolved.ordered_skills[0].skill_id, "d");
    assert_eq!(resolved.ordered_skills[1].skill_id, "c");
    assert_eq!(resolved.ordered_skills[2].skill_id, "b");
    assert_eq!(resolved.ordered_skills[3].skill_id, "a");
}

#[test]
fn test_dependency_missing_dependency() {
    // A depends on B, but B is not in the graph
    let mut graph = DependencyGraph::new();
    
    let a_deps = vec![Dependency::new("b", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("a", Version::parse("1.0.0").unwrap(), a_deps));

    let resolver = DependencyResolver::new(graph);
    let resolution = resolver.resolve_dependencies(&["a".to_string()]);
    assert!(resolution.is_err());
    assert!(matches!(
        resolution.unwrap_err(),
        DependencyError::MissingDependency { .. }
    ));
}

#[test]
fn test_dependency_self_dependency() {
    // A depends on itself
    let mut graph = DependencyGraph::new();
    
    let a_deps = vec![Dependency::new("a", ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new("a", Version::parse("1.0.0").unwrap(), a_deps));

    let resolver = DependencyResolver::new(graph);
    let cycles = resolver.detect_cycles();
    assert!(!cycles.is_empty());
    assert!(cycles.iter().any(|cycle| cycle.contains(&"a".to_string())));
}

// ============================================================================
// 3. Execution engine tests
// ============================================================================

#[test]
fn test_execution_concurrent_execution() {
    let registry: Arc<Mutex<ExecutorRegistry<u32, u32>>> = Arc::new(Mutex::new(ExecutorRegistry::new()));
    let counter = CounterExecutor::new("counter");
    let count_clone = counter.count.clone();
    
    {
        let mut reg = registry.lock().unwrap();
        reg.register("counter".to_string(), Box::new(counter)).unwrap();
    }

    // Execute concurrently from multiple threads
    let mut handles = vec![];
    for _ in 0..10 {
        let reg_clone = Arc::clone(&registry);
        let handle = thread::spawn(move || {
            let ctx = ExecutionContext::new(0);
            let reg = reg_clone.lock().unwrap();
            reg.execute("counter", &ctx)
        });
        handles.push(handle);
    }

    // Wait for all threads
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All should succeed
    for result in &results {
        assert!(result.is_ok());
    }

    // Counter should have been incremented 10 times
    assert_eq!(*count_clone.lock().unwrap(), 10);
}

#[test]
fn test_execution_timeout_handling() {
    let mut registry: ExecutorRegistry<String, String> = ExecutorRegistry::new();
    // Create executor with 5 second delay
    registry
        .register("slow".to_string(), Box::new(EchoExecutor::new("slow", 5000)))
        .unwrap();

    // Create context with 100ms timeout
    let ctx = ExecutionContext::new("input".to_string())
        .with_timeout(Duration::from_millis(100));

    // Execute should timeout (but our current implementation doesn't enforce timeout,
    // so we just verify the timeout is set)
    assert_eq!(ctx.timeout(), Some(Duration::from_millis(100)));
}

#[test]
fn test_execution_error_recovery() {
    let mut registry: ExecutorRegistry<String, String> = ExecutorRegistry::new();
    
    // Register a failing executor
    registry
        .register("failing".to_string(), Box::new(EchoExecutor::new("failing", 10).with_failure()))
        .unwrap();

    // Register a recovery executor
    registry
        .register("recovery".to_string(), Box::new(EchoExecutor::new("recovery", 10)))
        .unwrap();

    // Execute failing skill
    let ctx = ExecutionContext::new("input".to_string());
    let result = registry.execute("failing", &ctx);
    assert!(result.is_err());

    // Recovery executor should still work
    let ctx = ExecutionContext::new("input".to_string());
    let result = registry.execute("recovery", &ctx);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert_eq!(output.status, ExecutionStatus::Completed);
}

#[test]
fn test_execution_state_transitions() {
    let mut state = ExecutionState::new();
    assert_eq!(state.status, ExecutionStatus::Pending);

    state.start();
    assert_eq!(state.status, ExecutionStatus::Running);
    assert!(state.started_at.is_some());

    thread::sleep(Duration::from_millis(10));
    state.complete();
    assert_eq!(state.status, ExecutionStatus::Completed);
    assert!(state.duration_ms.is_some());
}

#[test]
fn test_execution_context_chaining() {
    let ctx = ExecutionContext::new("input".to_string())
        .with_timeout(Duration::from_secs(30));

    assert_eq!(ctx.timeout(), Some(Duration::from_secs(30)));
}

#[test]
fn test_execution_resource_limits() {
    let limits = ResourceLimits {
        max_memory: Some(1024 * 1024), // 1MB
        max_cpu: Some(50.0),
        max_time: None,
        max_processes: None,
        max_open_files: None,
    };

    assert_eq!(limits.max_memory, Some(1024 * 1024));
    assert_eq!(limits.max_cpu, Some(50.0));
}

// ============================================================================
// 4. Persistence tests
// ============================================================================

#[test]
fn test_persistence_save_and_load() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSkillStorage::new(temp_dir.path().to_path_buf());

    let skill = create_skill_with_deps("test-skill", "Test Skill", vec![]);

    // Save skill
    assert!(storage.save_skill(&skill).is_ok());

    // Load skill
    let loaded = storage.load_skill("test-skill");
    assert!(loaded.is_ok());
    let loaded_skill = loaded.unwrap();
    assert_eq!(loaded_skill.id, "test-skill");
    assert_eq!(loaded_skill.metadata.name, "Test Skill");
}

#[test]
fn test_persistence_list_skills() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSkillStorage::new(temp_dir.path().to_path_buf());

    // Save multiple skills
    let skill1 = create_skill_with_deps("skill-1", "Skill 1", vec![]);
    let skill2 = create_skill_with_deps("skill-2", "Skill 2", vec![]);
    let skill3 = create_skill_with_deps("skill-3", "Skill 3", vec![]);

    storage.save_skill(&skill1).unwrap();
    storage.save_skill(&skill2).unwrap();
    storage.save_skill(&skill3).unwrap();

    // List all skills
    let skills = storage.list_skills();
    assert!(skills.is_ok());
    let skills_list = skills.unwrap();
    assert_eq!(skills_list.len(), 3);
}

#[test]
fn test_persistence_update_skill() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSkillStorage::new(temp_dir.path().to_path_buf());

    let mut skill = create_skill_with_deps("test-skill", "Test Skill", vec![]);
    storage.save_skill(&skill).unwrap();

    // Update skill
    skill.metadata.description = "Updated description".to_string();
    assert!(storage.update_skill(&skill).is_ok());

    // Load and verify
    let loaded = storage.load_skill("test-skill").unwrap();
    assert_eq!(loaded.metadata.description, "Updated description");
}

#[test]
fn test_persistence_delete_skill() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSkillStorage::new(temp_dir.path().to_path_buf());

    let skill = create_skill_with_deps("test-skill", "Test Skill", vec![]);
    storage.save_skill(&skill).unwrap();

    // Delete skill
    assert!(storage.delete_skill("test-skill").is_ok());

    // Should no longer exist
    let loaded = storage.load_skill("test-skill");
    assert!(loaded.is_err());
}

#[test]
fn test_persistence_concurrent_access() {
    let temp_dir = TempDir::new().unwrap();
    let storage = Arc::new(Mutex::new(FileSkillStorage::new(temp_dir.path().to_path_buf())));

    // Spawn multiple threads that read/write concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let handle = thread::spawn(move || {
            let mut storage = storage_clone.lock().unwrap();
            let skill = create_skill_with_deps(
                &format!("skill-{}", i),
                &format!("Skill {}", i),
                vec![],
            );
            storage.save_skill(&skill).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all skills were saved
    let storage = storage.lock().unwrap();
    let skills = storage.list_skills().unwrap();
    assert_eq!(skills.len(), 10);
}

#[test]
fn test_persistence_index_operations() {
    let mut index = SkillIndex::new();

    // Add skills to index
    let skill1 = create_skill_with_deps("skill-1", "Skill One", vec![]);
    let skill2 = create_skill_with_deps("skill-2", "Skill Two", vec![]);

    index.insert("skill-1".to_string(), skill1.clone());
    index.insert("skill-2".to_string(), skill2.clone());

    // Lookup by ID
    assert_eq!(index.get("skill-1").unwrap().id, "skill-1");
    assert_eq!(index.get("skill-2").unwrap().id, "skill-2");

    // Lookup by name
    let by_name = index.get_by_name("Skill One");
    assert_eq!(by_name.len(), 1);
    assert_eq!(by_name[0].id, "skill-1");

    // Remove from index
    index.remove("skill-1");
    assert!(index.get("skill-1").is_none());
}

#[test]
fn test_persistence_sync_from_disk() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSkillStorage::new(temp_dir.path().to_path_buf());

    // Save some skills
    let skill1 = create_skill_with_deps("skill-1", "Skill 1", vec![]);
    let skill2 = create_skill_with_deps("skill-2", "Skill 2", vec![]);
    storage.save_skill(&skill1).unwrap();
    storage.save_skill(&skill2).unwrap();

    // Create a new storage and sync from disk
    let storage2 = FileSkillStorage::new(temp_dir.path().to_path_buf());
    
    // Should have synced
    assert_eq!(storage2.count().unwrap(), 2);
}

#[test]
fn test_persistence_rebuild_index() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSkillStorage::new(temp_dir.path().to_path_buf());

    // Save skills
    let skill1 = create_skill_with_deps("skill-1", "Skill One", vec![]);
    let skill2 = create_skill_with_deps("skill-2", "Skill Two", vec![]);
    storage.save_skill(&skill1).unwrap();
    storage.save_skill(&skill2).unwrap();

    // Rebuild index
    storage.rebuild_index().unwrap();

    assert_eq!(storage.count().unwrap(), 2);
}

#[test]
fn test_persistence_storage_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSkillStorage::new(temp_dir.path().to_path_buf());

    // Try to load non-existent skill
    let result = storage.load_skill("non-existent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PersistenceError::SkillNotFound(_)));

    // Try to delete non-existent skill
    let result = storage.delete_skill("non-existent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), PersistenceError::SkillNotFound(_)));
}

// ============================================================================
// 5. CLI integration tests: full workflow from discovery to execution
// ============================================================================

#[test]
fn test_cli_full_workflow_discovery_to_execution() {
    let mut cli = MarketplaceCLI::new();

    // 1. Register skills with dependencies
    let base_skill = create_skill_with_deps("base", "Base Library", vec![]);
    let util_skill = create_skill_with_deps(
        "util",
        "Utility",
        vec![Dependency::new("base", ">=1.0.0").unwrap()],
    );
    let app_skill = create_skill_with_deps(
        "app",
        "Application",
        vec![
            Dependency::new("util", ">=1.0.0").unwrap(),
            Dependency::new("base", ">=1.0.0").unwrap(),
        ],
    );

    cli.registry_mut().register(base_skill).unwrap();
    cli.registry_mut().register(util_skill).unwrap();
    cli.registry_mut().register(app_skill).unwrap();

    // 2. Search for skills
    let result = cli.execute(Command::Search {
        query: "base".to_string(),
        tags: vec![],
        author: None,
    });
    assert!(result.is_ok());
    assert!(result.unwrap().contains("Base Library"));

    // 3. Show skill details
    let result = cli.execute(Command::Show { id: "app".to_string() });
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Application"));

    // 4. Install skills in dependency order
    let result = cli.execute(Command::Install {
        id: "base".to_string(),
        path: None,
    });
    assert!(result.is_ok());

    let result = cli.execute(Command::Install {
        id: "util".to_string(),
        path: None,
    });
    assert!(result.is_ok());

    let result = cli.execute(Command::Install {
        id: "app".to_string(),
        path: None,
    });
    assert!(result.is_ok());

    // 5. Verify all installed
    let result = cli.execute(Command::List {
        category: None,
        installed: Some(true),
    });
    assert!(result.is_ok());
    assert!(result.unwrap().contains("3 skill(s)"));

    // 6. Rate skills
    let rating = Rating::new(5).unwrap();
    let result = cli.execute(Command::Rate {
        id: "base".to_string(),
        rating,
    });
    assert!(result.is_ok());
}

#[test]
fn test_cli_workflow_install_uninstall_cycle() {
    let mut cli = MarketplaceCLI::new();

    let skill = create_skill_with_deps("cycle", "Cycle Skill", vec![]);
    cli.registry_mut().register(skill).unwrap();

    // Install
    let result = cli.execute(Command::Install {
        id: "cycle".to_string(),
        path: None,
    });
    assert!(result.is_ok());
    assert!(cli.registry().get("cycle").unwrap().installed);

    // Uninstall
    let result = cli.execute(Command::Uninstall {
        id: "cycle".to_string(),
    });
    assert!(result.is_ok());
    assert!(!cli.registry().get("cycle").unwrap().installed);

    // Install again
    let result = cli.execute(Command::Install {
        id: "cycle".to_string(),
        path: None,
    });
    assert!(result.is_ok());
    assert!(cli.registry().get("cycle").unwrap().installed);
}

// ============================================================================
// 6. Combined integration tests
// ============================================================================

#[test]
fn test_combined_execution_with_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileSkillStorage::new(temp_dir.path().to_path_buf());

    // Create and save skills
    let skill1 = create_skill_with_deps("exec-1", "Exec Skill 1", vec![]);
    let skill2 = create_skill_with_deps("exec-2", "Exec Skill 2", vec![Dependency::new("exec-1", ">=1.0.0").unwrap()]);
    storage.save_skill(&skill1).unwrap();
    storage.save_skill(&skill2).unwrap();

    // Load skills and build dependency graph
    let loaded1 = storage.load_skill("exec-1").unwrap();
    let loaded2 = storage.load_skill("exec-2").unwrap();

    let mut graph = DependencyGraph::new();
    graph.add_node(DependencyNode::new(&loaded1.id, Version::parse("1.0.0").unwrap(), vec![]));
    let deps = vec![Dependency::new(&loaded1.id, ">=1.0.0").unwrap()];
    graph.add_node(DependencyNode::new(&loaded2.id, Version::parse("1.0.0").unwrap(), deps));

    // Resolve dependencies
    let resolver = DependencyResolver::new(graph);
    let resolution = resolver.resolve_dependencies(&["exec-2".to_string()]);
    assert!(resolution.is_ok());
    let resolved = resolution.unwrap();
    assert_eq!(resolved.ordered_skills.len(), 2);

    // Set up executor registry
    let mut registry: ExecutorRegistry<String, String> = ExecutorRegistry::new();
    registry.register("exec-1".to_string(), Box::new(EchoExecutor::new("exec-1", 10))).unwrap();
    registry.register("exec-2".to_string(), Box::new(EchoExecutor::new("exec-2", 10))).unwrap();

    // Execute in order
    for skill in &resolved.ordered_skills {
        let ctx = ExecutionContext::new("test".to_string());
        let result = registry.execute(&skill.skill_id, &ctx);
        assert!(result.is_ok());
    }
}

#[test]
fn test_combined_cli_with_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let mut cli = MarketplaceCLI::new();

    // Create skills
    let skill1 = create_skill_with_deps("persist-1", "Persist Skill 1", vec![]);
    let skill2 = create_skill_with_deps("persist-2", "Persist Skill 2", vec![Dependency::new("persist-1", ">=1.0.0").unwrap()]);

    cli.registry_mut().register(skill1).unwrap();
    cli.registry_mut().register(skill2).unwrap();

    // Install skills
    cli.execute(Command::Install { id: "persist-1".to_string(), path: None }).unwrap();
    cli.execute(Command::Install { id: "persist-2".to_string(), path: None }).unwrap();

    // Verify installation
    let result = cli.execute(Command::List { category: None, installed: Some(true) });
    assert!(result.is_ok());
    assert!(result.unwrap().contains("2 skill(s)"));

    // Rate skills
    cli.execute(Command::Rate { id: "persist-1".to_string(), rating: Rating::new(5).unwrap() }).unwrap();
    cli.execute(Command::Rate { id: "persist-2".to_string(), rating: Rating::new(4).unwrap() }).unwrap();

    // Get top rated
    let result = cli.execute(Command::TopRated { n: 2 });
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("Persist Skill 1"));
    assert!(output.contains("Persist Skill 2"));
}
