# Scripts

Utility scripts for development and deployment.

## Available Scripts

### Development

```bash
# Run all development servers
./scripts/dev.sh

# Run specific app
./scripts/dev.sh desktop
./scripts/dev.sh marketplace
```

### Build

```bash
# Build all projects
./scripts/build.sh

# Build for release
./scripts/build.sh --release
```

### Testing

```bash
# Run all tests
./scripts/test.sh

# Run with coverage
./scripts/test.sh --coverage
```

### Deployment

```bash
# Deploy to production
./scripts/deploy.sh --target production

# Deploy to staging
./scripts/deploy.sh --target staging
```

## Script Details

- `dev.sh` - Starts development servers for all apps
- `build.sh` - Builds all projects for production
- `test.sh` - Runs test suite
- `deploy.sh` - Deploys applications
- `clean.sh` - Cleans build artifacts
- `setup.sh` - Initial project setup
