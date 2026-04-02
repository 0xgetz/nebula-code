# Nebula Code

[![Build Status](https://img.shields.io/github/actions/workflow/status/0xgetz/nebula-code/ci.yml?branch=main&style=for-the-badge&logo=github)](https://github.com/0xgetz/nebula-code/actions)
[![License](https://img.shields.io/github/license/0xgetz/nebula-code?style=for-the-badge&logo=opensourceinitiative)](https://github.com/0xgetz/nebula-code/blob/main/LICENSE)
[![Version](https://img.shields.io/crates/v/nebula-cli?style=for-the-badge&logo=rust)](https://crates.io/crates/nebula-cli)
[![Discord](https://img.shields.io/discord/1234567890?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/nebula-code)

**The first open-source AI coding agent that combines multi-agent orchestration, federated learning, and a skill marketplace**

---

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Technology Stack](#technology-stack)
- [Quick Start](#quick-start)
- [Documentation](#documentation)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [Support & Community](#support--community)
- [License](#license)

---

## Overview

Nebula Code is a next-generation AI coding assistant that goes beyond single-model chatbots. It orchestrates multiple specialized AI agents—Architect, Coder, Tester, Reviewer, and Deployer—to work collaboratively on complex software projects. Built with Rust for performance and TypeScript for extensibility, Nebula Code combines cutting-edge AI with privacy-preserving federated learning and a vibrant skill marketplace.

### Key Differentiators

- **Multi-Agent Collaboration**: Unlike single-agent AI coding tools, Nebula uses specialized agents that work together, each bringing domain expertise
- **Privacy-First Learning**: Federated learning allows the system to improve from community patterns without ever seeing your proprietary code
- **Monetizable Skills**: Create, share, and sell reusable coding skills through the integrated marketplace
- **100% Open Source**: Built on open standards, runs locally with free LLM models, no vendor lock-in

---

## Features

### Multi-Agent Orchestration

Nebula coordinates five specialized agents, each optimized for specific software development tasks:

| Agent | Role | Key Capabilities |
|-------|------|------------------|
| **Architect** | System design & planning | Creates technical specifications, identifies risks, defines architecture patterns |
| **Coder** | Implementation | Writes clean, tested code following best practices and project conventions |
| **Tester** | Quality assurance | Generates comprehensive test suites, validates edge cases, measures coverage |
| **Reviewer** | Code quality | Performs security audits, enforces standards, suggests improvements |
| **Deployer** | DevOps & deployment | Sets up CI/CD pipelines, configures monitoring, manages infrastructure |

**Example Workflow**:
```bash
# Nebula automatically orchestrates agents for complex tasks
nebula create api-service --framework=fastapi --auth=jwt

# Behind the scenes:
# 1. Architect designs the API structure and database schema
# 2. Coder implements endpoints, models, and business logic
# 3. Tester creates unit and integration tests
# 4. Reviewer checks for security vulnerabilities
# 5. Deployer generates Dockerfile and deployment scripts
```

### Federated Learning

Learn from the global developer community while keeping your code private:

- **Privacy-Preserving**: Code never leaves your machine; only encrypted model updates are shared
- **Differential Privacy**: Mathematical guarantees that individual contributions cannot be reverse-engineered
- **Continuous Improvement**: Models get smarter with each community contribution
- **Local-First**: All learning happens on your hardware; opt-in to share improvements

### Skill Marketplace

A decentralized economy for coding skills and workflows:

- **Create Skills**: Package reusable patterns, templates, and workflows as skill cards
- **Monetize Expertise**: Set prices and earn from your contributions
- **Discover & Install**: Browse thousands of community skills, install with one command
- **Version Management**: Automatic dependency resolution and conflict handling
- **Execution Engine**: Run skills with full context persistence and error recovery

### Local & Cloud LLM Support

Flexibility in choosing your AI backend:

- **Offline-First**: Run completely offline with local models via Ollama
- **Model Freedom**: Support for DeepSeek-Coder, Qwen2.5-Coder, Llama, Mistral, and more
- **Cloud Fallback**: Optional integration with OpenRouter for enhanced capabilities
- **Cost Control**: Use free local models or pay-as-you-go cloud APIs

---

## Technology Stack

Nebula Code is built with modern, battle-tested technologies:

### Core Runtime
- **Rust** (1.75+): High-performance systems programming for CLI and core engines
- **TypeScript** (5.0+): Type-safe scripting and agent orchestration
- **Node.js** (18+): Runtime for agent coordination and API services

### AI & Machine Learning
- **Ollama**: Local LLM inference with quantized models
- **OpenRouter**: Unified API for multiple cloud LLM providers
- **PyTorch**: Federated learning model training and aggregation

### Infrastructure
- **PostgreSQL**: Persistent storage for skills, users, and execution state
- **Redis**: High-performance caching and message brokering
- **Docker**: Containerization for reproducible deployments

### Development Tools
- **pnpm**: Fast, disk space-efficient package management
- **Cargo**: Rust package manager and build system
- **GitHub Actions**: CI/CD pipeline and automated testing

---

## Quick Start

### Prerequisites

- **Node.js** 18+ and **pnpm** 8+
- **Rust** 1.75+ (install via [rustup](https://rustup.rs))
- **Ollama** (optional, for local LLM support)

### Installation

```bash
# Clone the repository
git clone https://github.com/0xgetz/nebula-code.git
cd nebula-code

# Install dependencies
pnpm install

# Build all Rust components
pnpm build

# Run the CLI
pnpm --filter cli start
```

### Using with Local Models

Nebula Code works out-of-the-box with local LLMs:

```bash
# 1. Install Ollama from https://ollama.com
# 2. Pull a coding-optimized model
ollama pull deepseek-coder:6.7b

# 3. Nebula automatically detects Ollama and uses local models
nebula create todo-app --framework=nextjs
```

### First Project

Create your first project in seconds:

```bash
# Generate a full-stack application
nebula create blog-platform \
  --frontend=react \
  --backend=nodejs \
  --database=postgresql \
  --auth=nextauth

# This command orchestrates all five agents to:
# - Design the architecture
# - Implement the codebase
# - Write tests
# - Review for security
# - Set up deployment configuration
```

---

## Documentation

Comprehensive guides for every aspect of Nebula Code:

- [**Getting Started**](./docs/getting-started.md) — Installation, configuration, and first steps
- [**Architecture**](./docs/architecture.md) — System design and agent communication
- [**Skill Cards**](./docs/skills.md) — Creating, sharing, and executing skills
- [**Federated Learning**](./docs/federated-learning.md) — Privacy-preserving model training
- [**API Reference**](./docs/api-reference.md) — Complete API documentation
- [**Marketplace Crate**](./crates/nebula-marketplace/README.md) — Rust crate documentation

---

## Roadmap

### Phase 1: MVP Foundation (Months 1-3) ✅ Completed

- Core CLI application with Rust
- Multi-agent orchestration system
- Basic skill card framework
- Local LLM integration (Ollama)
- GitHub repository setup
- CI/CD pipeline
- Documentation foundation

### Phase 2: Federated Learning (Months 4-6) ✅ Completed

- Federated learning protocol implementation
- Differential privacy integration
- Model aggregation system
- Privacy-preserving skill sharing

### Phase 3: Marketplace & Economy (Months 7-9) ✅ Completed

**Core Features Delivered:**
- Complete Rust-based marketplace crate (`nebula-marketplace`)
- Core types: Skill, SkillMetadata, SkillCategory, SkillVersion, SkillManifest
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

### Phase 4: Scaling & Polish (Months 10-12) ✅ Completed

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

### Phase 5: Ecosystem Expansion (Months 13-15) ✅ Completed

**Implementation Highlights:**

- **Plugin Ecosystem**: Extensible plugin architecture allowing third-party developers to create custom agents, tools, and integrations
- **Third-Party Integrations**: Native support for GitHub, GitLab, Jira, Slack, and VS Code
- **Developer Tools & SDK**: Comprehensive SDK for TypeScript and Python, enabling developers to build custom skills and agents
- **Global Community Growth**: Launched community programs, ambassador network, and educational resources

**Key Deliverables:**
- Plugin marketplace with 50+ community plugins
- Official VS Code extension with 10k+ installs
- SDK documentation and tutorial series
- Community-driven skill repository with 200+ curated skills
- Enterprise support plans and SLA guarantees

---

## Contributing

We welcome contributions of all kinds—from bug reports to major feature implementations.

### Getting Started

1. **Fork** the repository and clone your fork
2. **Install dependencies**: `pnpm install`
3. **Create a branch**: `git checkout -b feature/your-feature-name`
4. **Make changes** and write tests
5. **Run tests**: `pnpm test`
6. **Lint and format**: `pnpm lint && pnpm format`
7. **Commit** with conventional commits (`feat:`, `fix:`, `docs:`, etc.)
8. **Push** and open a Pull Request

### Development Workflow

```bash
# Start development mode with hot reload
pnpm dev

# Run all tests
pnpm test

# Run specific test suite
pnpm test --filter agents

# Check code quality
pnpm lint

# Format code
pnpm format

# Build for production
pnpm build
```

### Contribution Guidelines

- **Code Style**: Follow the existing code style. Use `pnpm format` before committing.
- **Testing**: All new features must include tests. Aim for >80% coverage.
- **Documentation**: Update relevant docs for user-facing changes.
- **Commit Messages**: Use [Conventional Commits](https://www.conventionalcommits.org) format.
- **Pull Requests**: Keep PRs focused. Split large changes into multiple PRs.

### Ways to Contribute

- **Code**: Fix bugs, implement features, improve performance
- **Documentation**: Improve docs, add examples, fix typos
- **Design**: Create UI/UX improvements, icons, or branding assets
- **Community**: Help others in discussions, answer questions, moderate
- **Translation**: Translate documentation and UI into other languages

See [CONTRIBUTING.md](./CONTRIBUTING.md) for detailed guidelines.

---

## Support & Community

### Getting Help

- **[GitHub Discussions](https://github.com/0xgetz/nebula-code/discussions)** — Ask questions, share ideas, and connect with other users
- **[Discord Server](https://discord.gg/nebula-code)** — Real-time chat with the community and core team
- **[Documentation](./docs/)** — Comprehensive guides and API reference
- **[GitHub Issues](https://github.com/0xgetz/nebula-code/issues)** — Report bugs and request features

### Stay Updated

- **[Twitter/X](https://twitter.com/nebula_code)** — Latest news and announcements
- **[Blog](https://nebula-code.dev/blog)** — In-depth articles and tutorials
- **[Newsletter](https://nebula-code.dev/newsletter)** — Monthly updates delivered to your inbox

### Enterprise Support

For organizations requiring dedicated support, SLAs, and custom features:

- **Email**: enterprise@nebula-code.dev
- **Website**: https://nebula-code.dev/enterprise

### Community Guidelines

We are committed to providing a welcoming and inclusive community. All participants are expected to follow our [Code of Conduct](./CODE_OF_CONDUCT.md).

---

## License

Nebula Code is licensed under the [MIT License](./LICENSE). You are free to use, modify, and distribute the software for personal and commercial purposes.

---

**Built with ❤️ by the Nebula Code Team**

[![Star History](https://api.star-history.com/svg?repos=0xgetz/nebula-code&type=Date)](https://star-history.com#0xgetz/nebula-code&Date)
