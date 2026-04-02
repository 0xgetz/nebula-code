# Nebula Code Architecture

This document provides an overview of Nebula Code's system architecture and design decisions.

## System Overview

Nebula Code is built as a modular monorepo with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                    Nebula Code Platform                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Desktop   │  │     CLI     │  │ Marketplace │         │
│  │   (Tauri)   │  │   (Rust)    │  │  (Next.js)  │         │
│  └─────┬───────┘  └─────┬───────┘  └─────┬───────┘         │
│        │                │                │                 │
│        └────────────────┼────────────────┘                 │
│                         │                                  │
│                ┌────────▼────────┐                        │
│                │  Shared Crates  │                        │
│                │  & Packages     │                        │
│                └────────┬────────┘                        │
│                         │                                  │
│        ┌────────────────┼────────────────┐               │
│        │                │                │               │
│  ┌─────▼───────┐  ┌─────▼───────┐  ┌─────▼───────┐       │
│  │   Agents    │  │   Skills    │  │   Models    │       │
│  │   System    │  │   System    │  │ Integration │       │
│  └─────────────┘  └─────────────┘  └─────────────┘       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Multi-Agent System

The agent system is the heart of Nebula Code, consisting of specialized agents:

- **Architect Agent**: Designs system architecture and creates implementation plans
- **Coder Agent**: Writes production-ready code following best practices
- **Tester Agent**: Generates comprehensive test suites and validates code quality
- **Reviewer Agent**: Performs security audits and code reviews
- **Deployer Agent**: Handles CI/CD, deployment, and monitoring setup

Agents communicate through a shared context and can be orchestrated in workflows.

### 2. Skill Card System

Skills are reusable coding patterns and workflows that can be:

- Created by developers
- Shared with the community
- Sold on the marketplace
- Installed locally

Each skill card contains:

- Metadata (name, description, version, author)
- Files (code, configuration, documentation)
- Dependencies
- Compatibility information

### 3. Federated Learning

Privacy-preserving learning system that:

- Extracts code patterns without exposing raw code
- Uses differential privacy for aggregation
- Continuously improves models through community contributions
- Maintains local model instances

### 4. Model Integration

Supports multiple LLM providers:

- Local models via Ollama (DeepSeek-Coder, Qwen2.5-Coder, Llama)
- Cloud models via OpenRouter (multi-model routing)
- Future: Claude, GPT, Gemini direct integration

## Technology Stack

### Frontend

- **Desktop**: Tauri 2.0 (Rust + React)
- **Web Marketplace**: Next.js 14 (React + TypeScript)
- **State Management**: Zustand
- **Styling**: Tailwind CSS

### Backend

- **CLI**: Rust with Clap
- **Shared Libraries**: Rust crates
- **Database**: SQLite (local), PostgreSQL (marketplace)
- **API**: REST + WebSocket

### Infrastructure

- **CI/CD**: GitHub Actions
- **Hosting**: Vercel (web), GitHub Releases (desktop)
- **Storage**: IPFS (decentralized skill storage)
- **Payments**: Stripe + Crypto (USDC)

## Data Flow

### Typical User Workflow

1. User runs `nebula init my-project`
2. Architect agent creates a plan based on requirements
3. User reviews and approves the plan
4. Coder agent generates code using selected skill cards
5. Tester agent creates test suites
6. Reviewer agent performs security audit
7. Deployer agent sets up CI/CD

### Skill Card Installation

1. User browses marketplace or local store
2. User purchases or downloads skill
3. Skill is validated and installed to local store
4. Skill becomes available for use in projects

## Security Considerations

- All code execution happens in sandboxed environments
- Skill cards are verified before installation
- API keys and credentials are encrypted
- Federated learning uses differential privacy
- Regular security audits and dependency updates

## Performance Optimizations

- Local model caching for faster inference
- Incremental builds for large projects
- Parallel agent execution when possible
- Efficient pattern matching for skill selection

## Future Enhancements

- Mobile app support (Tauri 2.0 mobile)
- Advanced code completion with local models
- Real-time collaboration features
- Plugin system for extensibility
- Enterprise features (SSO, team management)
