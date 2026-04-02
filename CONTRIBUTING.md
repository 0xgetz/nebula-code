# Contributing to Nebula Code

Thank you for your interest in contributing to Nebula Code! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please read and follow our [Code of Conduct](./CODE_OF_CONDUCT.md) to maintain a welcoming and inclusive community.

## Getting Started

### Prerequisites

- Node.js 18+ and pnpm 8+
- Rust 1.75+
- Git

### Development Setup

1. Fork the repository
2. Clone your fork:
   ```bash
   git clone https://github.com/your-username/nebula-code.git
   cd nebula-code
   ```
3. Install dependencies:
   ```bash
   pnpm install
   ```
4. Start development:
   ```bash
   pnpm dev
   ```

## Project Structure

```
nebula-code/
├── apps/
│   ├── desktop/      # Tauri desktop application
│   ├── cli/          # Rust CLI tool
│   └── marketplace/  # Next.js web marketplace
├── crates/           # Shared Rust libraries
├── packages/         # Shared TypeScript packages
├── docs/             # Documentation
└── tests/            # Integration tests
```

## Development Workflow

### Making Changes

1. Create a new branch:
   ```bash
   git checkout -b feature/your-feature-name
   ```
2. Make your changes
3. Run tests:
   ```bash
   pnpm test
   ```
4. Lint your code:
   ```bash
   pnpm lint
   ```
5. Format your code:
   ```bash
   pnpm format
   ```

### Committing Changes

We use conventional commits:
- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `refactor:` Code refactoring
- `test:` Test additions or changes
- `chore:` Maintenance tasks

Example:
```bash
git commit -m "feat(cli): add new build command with caching"
```

### Pull Requests

1. Push your branch to GitHub
2. Open a Pull Request
3. Fill out the PR template
4. Request review from maintainers
5. Address review feedback
6. Once approved, your PR will be merged

## Testing

### Unit Tests

```bash
# Run all tests
pnpm test

# Run tests for specific package
pnpm --filter cli test
```

### Integration Tests

```bash
# Run integration tests
pnpm test:integration
```

### Manual Testing

For CLI changes:
```bash
pnpm --filter cli start -- [command] [args]
```

## Documentation

- Update documentation for user-facing changes
- Add inline comments for complex logic
- Update README.md for significant changes

## Skill Cards

If you're creating a skill card:

1. Follow the [Skill Card Specification](./docs/skills.md)
2. Include comprehensive tests
3. Add usage examples
4. Document dependencies and requirements

## Federated Learning Contributions

When contributing to federated learning components:

1. Ensure privacy-preserving properties are maintained
2. Add tests for differential privacy guarantees
3. Document any changes to aggregation algorithms

## Questions?

- Check existing [documentation](./docs/)
- Search [GitHub Discussions](https://github.com/0xgetz/nebula-code/discussions)
- Ask in our [Discord community](https://discord.gg/nebula-code)

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](./LICENSE).
