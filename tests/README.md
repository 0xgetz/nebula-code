# Tests

Test suite for Nebula Code.

## Running Tests

```bash
# Run all tests
pnpm test

# Run tests with coverage
pnpm test:coverage

# Run specific test suite
pnpm test -- --testPathPattern=cli
```

## Test Structure

```
tests/
├── integration/     # Integration tests
├── e2e/            # End-to-end tests
└── fixtures/       # Test data and fixtures
```

## Writing Tests

### Unit Tests

Place unit tests alongside source files using `.test.ts` or `.test.rs` naming convention.

### Integration Tests

Integration tests live in the `tests/integration/` directory and test interactions between components.

### End-to-End Tests

E2E tests in `tests/e2e/` test complete user workflows.

## Test Commands

- `pnpm test` - Run all tests
- `pnpm test:watch` - Run tests in watch mode
- `pnpm test:coverage` - Generate coverage report
- `pnpm test:update` - Update snapshots
