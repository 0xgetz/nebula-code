# Nebula Code

**AI Coding Agent with Federated Learning, Skill Marketplace, and Multi-Agent Orchestration**

Nebula Code is the first open-source AI coding agent that combines:

- **Multi-Agent System**: Specialized agents (Architect, Coder, Tester, Reviewer, Deployer) working together
- **Federated Learning**: Privacy-preserving code pattern sharing - learn from the community without exposing your code
- **Skill Marketplace**: Create, share, and monetize coding skills and workflows
- **100% Free & Open Source**: Built on open standards, runs locally with free LLM models

## Features

### Multi-Agent Orchestration
- **Architect**: Designs system architecture and creates implementation plans
- **Coder**: Writes production-ready code following best practices
- **Tester**: Generates comprehensive test suites and validates code quality
- **Reviewer**: Performs security audits and code reviews
- **Deployer**: Handles CI/CD, deployment, and monitoring setup

### Federated Learning
- Learn from global coding patterns without sharing raw code
- Differential privacy ensures your code never leaves your machine
- Continuously improving models through community contributions

### Skill Marketplace
- Create reusable skill cards for common patterns and workflows
- Monetize your expertise with one-time purchases
- Discover and install skills from the community
- Advanced execution engine with dependency resolution and persistence

### Local & Cloud LLM Support
- Run completely offline with local models (Ollama)
- Support for DeepSeek-Coder, Qwen2.5-Coder, Llama, and more
- Optional cloud integration via OpenRouter for enhanced capabilities

## Recent Updates

- **2026-04-02**: Phase 3 (Marketplace & Economy) completed! Implemented advanced features including skill execution engine, dependency resolution system, and persistence layer in the `nebula-marketplace` Rust crate.

## Quick Start

### Prerequisites

- Node.js 18+ and pnpm 8+
- Rust 1.75+
- (Optional) Ollama for local LLM support

### Installation

```bash
# Clone the repository
git clone https://github.com/0xgetz/nebula-code.git
cd nebula-code

# Install dependencies
pnpm install

# Build the project
pnpm build

# Run the CLI
pnpm --filter cli start
```

### Using with Local Models

1. Install [Ollama](https://ollama.com)
2. Pull a coding model:
   ```bash
   ollama pull deepseek-coder:6.7b
   ```
3. Nebula Code will auto-detect Ollama and use local models by default

## Documentation

- [Getting Started](./docs/getting-started.md)
- [Architecture](./docs/architecture.md)
- [Skill Cards](./docs/skills.md)
- [Federated Learning](./docs/federated-learning.md)
- [API Reference](./docs/api-reference.md)
- [Marketplace Crate](./crates/nebula-marketplace/README.md)

## Roadmap

### Phase 1: MVP Foundation (Months 1-3) ✅
- [x] Core CLI application with Rust
- [x] Multi-agent orchestration system
- [x] Basic skill card framework
- [x] Local LLM integration (Ollama)
- [x] GitHub repository setup
- [x] CI/CD pipeline
- [x] Documentation foundation

### Phase 2: Federated Learning (Months 4-6) ✅
- [x] Federated learning protocol implementation
- [x] Differential privacy integration
- [x] Model aggregation system
- [x] Privacy-preserving skill sharing

### Phase 3: Marketplace & Economy (Months 7-9) ✅
- [x] Skill marketplace platform
- [x] Payment integration (crypto + fiat)
- [x] Creator economy features
- [x] Skill rating and discovery

**Implemented:** Complete Rust-based marketplace crate (`nebula-marketplace`) with:

**Core Features:**
- Core types (Skill, SkillMetadata, SkillCategory, SkillVersion, SkillManifest)
- Registry system for storing and querying skills
- Discovery traits for searching by category/tags/name
- Skill installation/uninstallation logic
- Rating and review system with aggregation
- CLI interface for browsing and managing skills

**Advanced Features (Phase 3.5):**
- **Skill Execution Engine**: Generic executor framework with `SkillExecutor` trait, `ExecutionContext`, `ExecutorRegistry`, and comprehensive error handling
- **Dependency Resolution**: Full dependency graph with cycle detection, version conflict resolution, topological sorting, and transitive dependency handling
- **Persistence Layer**: Trait-based storage with file-based JSON backend, in-memory indexing, and sync operations
- Comprehensive test coverage (100+ tests) and integration examples

### Phase 4: Scaling & Polish (Months 10-12) ✅
- [x] Performance optimization
- [x] Advanced agent capabilities
- [x] Enterprise features
- [x] Mobile and web interfaces

**Implemented:** Complete multi-agent collaboration system in the `nebula-agents` Rust crate with:

**Core Agent System:**
- Agent types and lifecycle management (Agent, AgentId, AgentState, AgentCapability, AgentMetadata)
- Communication protocol with typed messages and asynchronous messaging
- Agent registry for dynamic agent discovery and capability tracking
- Pub/sub communication channels for inter-agent messaging

**Orchestration Engine:**
- Task scheduling with priority-based ordering (Low, Normal, High, Critical)
- Dependency resolution and task graph management
- Task lifecycle management (Pending, Running, Completed, Failed, Cancelled)
- Agent assignment and load balancing

**Advanced Features:**
- Fault tolerance with agent failure detection and recovery
- Real-time task status tracking and monitoring
- Comprehensive test coverage with integration tests
- Example implementations demonstrating multi-agent workflows


### Phase 5: Ecosystem Expansion (Months 13-15)
- [ ] Plugin ecosystem
- [ ] Third-party integrations
- [ ] Developer tools and SDK
- [ ] Global community growth

## Contributing

We welcome contributions! Please read our [Contributing Guide](./CONTRIBUTING.md) and [Code of Conduct](./CODE_OF_CONDUCT.md) before getting started.

### Development

```bash
# Start development mode
pnpm dev

# Run tests
pnpm test

# Lint code
pnpm lint

# Format code
pnpm format
```

## Community

- [GitHub Discussions](https://github.com/0xgetz/nebula-code/discussions)
- [Discord](https://discord.gg/nebula-code)
- [Twitter](https://twitter.com/nebula_code)

## License

MIT License - see [LICENSE](./LICENSE) for details.

---

Built with ❤️ by the Nebula Code Team
