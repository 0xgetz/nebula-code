# Nebula Core

Core library providing fundamental types and functionality for Nebula Code.

## Features

- **Skill Card Model**: Complete data model for skill cards with validation
- **Project Management**: Project structure and configuration handling
- **Error Handling**: Comprehensive error types for all operations

## Usage

```rust
use nebula_core::{SkillCard, Project, NebulaError};

// Create a new skill card
let skill = SkillCard::new("my-skill", "My Skill", "A custom skill");

// Create a new project
let project = Project::new("my-project", PathBuf::from("."));
```

## API Reference

See the [documentation](https://docs.rs/nebula-core) for detailed API reference.
