# Nebula Marketplace

**Advanced Skill Marketplace for Nebula Code**

A comprehensive Rust crate providing a skill marketplace with execution engine, dependency resolution, and persistence capabilities.

## Features

### Core Marketplace
- **Skill Registry**: Centralized storage and discovery of skills
- **Skill Discovery**: Search by category, tags, name, or author
- **Rating & Reviews**: Community-driven skill quality assessment
- **CLI Interface**: Command-line tools for skill management

### Skill Execution Engine
- **Generic Executor Framework**: Type-safe execution with Rust generics
- **ExecutionContext**: Environment variables, configuration, and execution state
- **Executor Registry**: Manage multiple executors for different skill types
- **Error Handling**: Comprehensive error types with `thiserror`
- **Execution Tracking**: Status monitoring, progress reporting, and retry logic

### Dependency Resolution
- **Dependency Graph**: Build and analyze skill dependencies
- **Cycle Detection**: Identify circular dependencies
- **Version Conflict Resolution**: Find compatible versions across dependencies
- **Topological Sorting**: Determine correct installation order
- **Optional Dependencies**: Handle optional and transitive dependencies
- **Transitive Resolution**: Resolve full dependency trees

### Persistence Layer
- **Trait-Based Storage**: Pluggable backend architecture
- **File-Based Storage**: JSON file persistence with configurable directory
- **In-Memory Indexing**: Fast lookups by ID, name, category, author, and tags
- **Sync Operations**: Disk synchronization and index rebuilding
- **Concurrent Access**: Thread-safe operations with RwLock

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
nebula-marketplace = "0.1.0"
```

## Quick Start

### Using the Execution Engine

```rust
use nebula_marketplace::execution::{
    SkillExecutor, ExecutionContext, SkillInstance, ExecutionOutput,
    ExecutorRegistry, ExecutionStatus, SkillExecutorError, SkillInstanceConfig,
};
use std::collections::HashMap;

// Define a custom executor
#[derive(Debug)]
struct CodeFormatter;

impl SkillExecutor<String, String> for CodeFormatter {
    fn execute(&self, ctx: &ExecutionContext<String>) -> Result<ExecutionOutput<String>, SkillExecutorError> {
        // Format the code (simplified example)
        let formatted = format!("// Formatted code:\n{}", ctx.input());
        Ok(ExecutionOutput::success(ExecutionStatus::Completed, formatted))
    }

    fn validate(&self, ctx: &ExecutionContext<String>) -> Result<(), SkillExecutorError> {
        if ctx.input().is_empty() {
            return Err(SkillExecutorError::ValidationError("Code cannot be empty".to_string()));
        }
        Ok(())
    }

    fn prepare(&self, _instance: &mut SkillInstance<String, String>) -> Result<(), SkillExecutorError> {
        Ok(())
    }

    fn skill_type(&self) -> &str {
        "code-formatter"
    }
}

// Use the executor registry
fn main() -> Result<(), SkillExecutorError> {
    let mut registry = ExecutorRegistry::<String, String>::new();
    registry.register("code-formatter".to_string(), Box::new(CodeFormatter))?;

    // Create execution context with environment variables
    let mut env = HashMap::new();
    env.insert("RUST_FMT".to_string(), "true".to_string());
    let ctx = ExecutionContext::with_env("fn main() {}".to_string(), env)
        .with_timeout(std::time::Duration::from_secs(30))
        .with_mode("production".to_string());

    // Execute the skill
    let result = registry.execute("code-formatter", &ctx)?;
    
    if result.is_success() {
        println!("Formatted code: {:?}", result.value());
    }

    Ok(())
}
```

### Using Dependency Resolution

```rust
use nebula_marketplace::dependencies::{
    Dependency, DependencyGraph, DependencyNode, DependencyResolver,
    SkillMetadata,
};
use semver::Version;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build a dependency graph
    let mut graph = DependencyGraph::new();

    // Add skills with their dependencies
    graph.add_node(DependencyNode::new(
        "web-framework",
        Version::parse("2.0.0")?,
        vec![
            Dependency::new("http-client", ">=1.0.0")?,
            Dependency::new("json-parser", "^2.0")?,
        ],
    ));

    graph.add_node(DependencyNode::new(
        "http-client",
        Version::parse("1.5.0")?,
        vec![Dependency::new("tls-lib", ">=0.11")?],
    ));

    graph.add_node(DependencyNode::new(
        "json-parser",
        Version::parse("2.1.0")?,
        vec![],
    ));

    graph.add_node(DependencyNode::new(
        "tls-lib",
        Version::parse("0.11.0")?,
        vec![],
    ));

    // Register available versions for version resolution
    graph.register_versions("http-client", vec![
        Version::parse("1.0.0")?,
        Version::parse("1.5.0")?,
        Version::parse("2.0.0")?,
    ]);

    // Create resolver and detect cycles
    let resolver = DependencyResolver::new(graph);
    let cycles = resolver.detect_cycles();
    if !cycles.is_empty() {
        eprintln!("Circular dependencies detected!");
        return Err("Cycle detected".into());
    }

    // Resolve dependencies in order
    let result = resolver.resolve_dependencies(&["web-framework".to_string()])?;

    println!("Installation order:");
    for skill in &result.ordered_skills {
        println!("  - {} v{}", skill.skill_id, skill.version);
    }

    // Check for version conflicts
    let conflicts = resolver.find_conflicts();
    if !conflicts.is_empty() {
        eprintln!("Version conflicts detected:");
        for conflict in &conflicts {
            eprintln!("  - {}: {:?}", conflict.dependency, conflict.requirements);
        }
    }

    Ok(())
}
```

### Using the Persistence Layer

```rust
use nebula_marketplace::persistence::{FileSkillStorage, SkillStorage, SkillIndex};
use nebula_marketplace::types::{Skill, SkillMetadata, SkillManifest, SkillVersion};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create file-based storage
    let storage = FileSkillStorage::new(PathBuf::from("./skills"));

    // Create a skill
    let metadata = SkillMetadata::new(
        "rust-linter".to_string(),
        "A comprehensive Rust linter".to_string(),
        "nebula-team".to_string(),
    )
    .with_version(SkillVersion::new(1, 2, 0))
    .with_categories(vec![nebula_marketplace::types::SkillCategory::CodeReview])
    .with_tags(vec!["rust".to_string(), "lint".to_string(), "quality".to_string()]);

    let manifest = SkillManifest::new("main.rs".to_string(), "rust".to_string());
    let skill = Skill::new("rust-linter-1".to_string(), metadata, manifest);

    // Save the skill
    storage.save_skill(&skill)?;

    // Load the skill
    let loaded = storage.load_skill("rust-linter-1")?;
    println!("Loaded skill: {} v{}", loaded.metadata.name, loaded.metadata.version);

    // Search skills
    let all_skills = storage.list_skills()?;
    println!("Total skills: {}", all_skills.len());

    // Update the skill
    let mut updated = loaded.clone();
    updated.metadata.version = SkillVersion::new(1, 3, 0);
    storage.update_skill(&updated)?;

    // Delete the skill
    storage.delete_skill("rust-linter-1")?;

    Ok(())
}
```

## Architecture

### Module Structure

```
src/
├── lib.rs              # Library root with re-exports
├── types.rs            # Core types (Skill, SkillMetadata, etc.)
├── registry.rs         # Skill registry for storage and querying
├── discovery.rs        # Discovery traits for searching skills
├── rating.rs           # Rating and review system
├── cli.rs              # Command-line interface
├── execution.rs        # Skill execution engine
├── dependencies.rs     # Dependency resolution system
└── persistence.rs      # Storage layer with file backend
```

### Execution Engine Architecture

The execution engine provides a generic framework for running skills:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│ SkillExecutor   │◄───│ ExecutorRegistry │───►│ SkillInstance   │
│ Trait           │    │                  │    │                 │
├─────────────────┤    ├──────────────────┤    ├─────────────────┤
│ + execute()     │    │ + register()     │    │ + id            │
│ + validate()    │    │ + execute()      │    │ + config        │
│ + prepare()     │    │ + get()          │    │ + state         │
│ + cleanup()     │    │ + history()      │    │ + dependencies  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         ▲                         ▲                       ▲
         │                         │                       │
┌────────┴────────┐       ┌────────┴────────┐    ┌────────┴────────┐
│ ExecutionContext│       │ ExecutionOutput │    │ ExecutionState  │
├─────────────────┤       ├─────────────────┤    ├─────────────────┤
│ - input         │       │ - value         │    │ - status        │
│ - env_vars      │       │ - status        │    │ - started_at    │
│ - config        │       │ - error         │    │ - duration_ms   │
│ - timeout       │       │ - metadata      │    │ - progress      │
└─────────────────┘       └─────────────────┘    └─────────────────┘
```

### Dependency Resolution Algorithm

The resolver uses a depth-first approach with cycle detection:

1. **Cycle Detection**: Uses DFS with recursion stack tracking
2. **Topological Sort**: Kahn's algorithm for ordering
3. **Version Resolution**: Finds highest compatible version
4. **Conflict Detection**: Identifies incompatible version requirements
5. **Transitive Resolution**: Recursively resolves all dependencies

```
Input: Skill IDs to resolve
  │
  ▼
Check for cycles ──► If cycle found, return error
  │
  ▼
For each skill:
  ├─ Resolve dependencies recursively
  ├─ Check version compatibility
  └─ Add to resolved set
  │
  ▼
Topological sort of resolved skills
  │
  ▼
Return ordered list + warnings
```

### Persistence Architecture

```
┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐
│ SkillStorage     │◄───│ FileSkillStorage │───►│ SkillIndex       │
│ Trait            │    │                  │    │                  │
├──────────────────┤    ├──────────────────┤    ├──────────────────┤
│ + save_skill()   │    │ + sync_from_disk │    │ + by_id          │
│ + load_skill()   │    │ + sync_to_disk   │    │ + by_name        │
│ + delete_skill() │    │ + rebuild_index  │    │ + by_category    │
│ + list_skills()  │    │                  │    │ + by_author      │
│ + update_skill() │    │ JSON Files       │    │ + by_tag         │
└──────────────────┘    └──────────────────┘    └──────────────────┘
```

## API Reference

### Execution Module

| Type | Description |
|------|-------------|
| `SkillExecutor<I, O>` | Trait for executing skills with input type I and output type O |
| `ExecutionContext<I>` | Environment and parameters for skill execution |
| `SkillInstance<I, O>` | Represents a running skill with configuration |
| `ExecutionOutput<O>` | Result of skill execution with status and output |
| `ExecutionState` | Tracks execution progress, timing, and retries |
| `ExecutorRegistry<I, O>` | Manages executors for different skill types |
| `SkillExecutorError` | Error types for execution failures |
| `ExecutionStatus` | Enum for execution state (Pending, Running, Completed, etc.) |

### Dependencies Module

| Type | Description |
|------|-------------|
| `Dependency` | Represents a skill dependency with version requirements |
| `DependencyGraph` | Graph structure for skill dependencies |
| `DependencyNode` | Node in the dependency graph |
| `DependencyResolver` | Resolves dependencies and detects conflicts |
| `ResolutionResult` | Result of dependency resolution |
| `ResolvedSkill` | A skill with resolved version |
| `VersionConflict` | Represents version conflicts between dependencies |
| `DependencyError` | Error types for dependency resolution failures |

### Persistence Module

| Type | Description |
|------|-------------|
| `SkillStorage` | Trait for skill storage backends |
| `FileSkillStorage` | File-based JSON storage implementation |
| `SkillIndex` | In-memory index for fast skill lookups |
| `StorageConfig` | Configuration for storage backends |
| `PersistenceError` | Error types for persistence operations |

## Testing

Run the test suite:

```bash
cd crates/nebula-marketplace
cargo test
```

The crate includes comprehensive tests for:
- Execution engine (22 unit tests)
- Dependency resolution (30+ unit tests)
- Persistence layer (20+ unit tests)
- Integration tests in `tests/` directory

## Examples

See the `examples/` directory for complete working examples:
- `examples/execution_demo.rs` - Execution engine usage
- `examples/dependency_resolution.rs` - Dependency resolution demo
- `examples/persistence_demo.rs` - Persistence layer usage

## Roadmap

### Phase 3 (Current) - Advanced Features ✅

- [x] Skill Execution Engine with generic types
- [x] Dependency Resolution with cycle detection and version conflicts
- [x] Persistence Layer with file-based storage
- [x] Comprehensive test coverage
- [x] CLI integration for all new features

### Phase 4 - Scaling & Polish

- [ ] Performance optimization for large skill graphs
- [ ] Database backend support (PostgreSQL, SQLite)
- [ ] Distributed execution support
- [ ] Skill sandboxing and security isolation
- [ ] Advanced dependency resolution strategies

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Ensure all tests pass: `cargo test`
5. Submit a pull request

## License

MIT License - see [LICENSE](../../LICENSE) for details.

---

Part of the [Nebula Code](https://github.com/0xgetz/nebula-code) project.
